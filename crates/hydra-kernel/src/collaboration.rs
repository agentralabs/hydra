//! O15 Real-Time Collaboration — shadow worker, pair programming, active participation.
//! Three modes: Shadow (async suggestions), Pair (file watch + auto-test), Active (chat).

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

// ── Types ──

/// Collaboration mode.
#[derive(Debug, Clone, PartialEq)]
pub enum CollabMode {
    Off,
    Shadow,
    PairProgramming,
    Active,
}

impl CollabMode {
    pub fn label(&self) -> &'static str {
        match self { Self::Off => "off", Self::Shadow => "shadow", Self::PairProgramming => "pair", Self::Active => "active" }
    }
}

/// A suggestion from the shadow worker or pair programming sidecar.
#[derive(Debug, Clone)]
pub struct CollabSuggestion {
    pub source_file: String,
    pub suggestion: String,
    pub confidence: f64,
}

/// Collaboration state tracked across the session.
pub struct CollaborationState {
    pub mode: CollabMode,
    pub watch_dir: Option<PathBuf>,
    pub last_activity: Instant,
    pub idle_threshold_secs: u64,
    pub suggestions: Vec<CollabSuggestion>,
    pub pending_tests: Vec<String>,
    pub recent_changes: Vec<String>,
}

impl CollaborationState {
    pub fn new() -> Self {
        Self {
            mode: CollabMode::Off, watch_dir: None,
            last_activity: Instant::now(), idle_threshold_secs: 30,
            suggestions: Vec::new(), pending_tests: Vec::new(),
            recent_changes: Vec::new(),
        }
    }

    pub fn set_mode(&mut self, mode: CollabMode, dir: Option<&Path>) {
        self.mode = mode;
        self.watch_dir = dir.map(|d| d.to_path_buf());
        self.last_activity = Instant::now();
        eprintln!("hydra-collab: mode set to {}", self.mode.label());
    }
}

impl Default for CollaborationState {
    fn default() -> Self { Self::new() }
}

// ── Shadow Worker ──

/// Detect if user has been idle (>30s since last file change).
pub fn detect_idle(state: &CollaborationState) -> bool {
    state.last_activity.elapsed().as_secs() >= state.idle_threshold_secs
}

/// Generate a suggestion based on recent file changes and genome knowledge.
pub fn generate_suggestion(
    changes: &[String],
    genome: &hydra_genome::GenomeStore,
) -> Option<CollabSuggestion> {
    if changes.is_empty() { return None; }
    let recent = changes.last()?;
    // EC-15.5: Only suggest for code files
    if !is_code_file(recent) { return None; }
    // Query genome for relevant approaches
    let query = format!("coding {} best practices", file_language(recent));
    let matches = genome.query(&query);
    let entry = matches.first()?;
    if entry.effective_confidence() < 0.5 { return None; }
    Some(CollabSuggestion {
        source_file: recent.clone(),
        suggestion: entry.approach.steps.first().cloned().unwrap_or_default(),
        confidence: entry.effective_confidence(),
    })
}

// ── Pair Programming ──

/// Identify companion files for a source file (tests, docs).
pub fn identify_companion_files(changed: &Path) -> Vec<(String, String)> {
    let stem = changed.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = changed.extension().and_then(|s| s.to_str()).unwrap_or("");
    if stem.is_empty() { return Vec::new(); }
    let mut companions = Vec::new();
    match ext {
        "rs" => {
            companions.push((format!("tests/{stem}_test.rs"), "test".into()));
        }
        "ts" | "tsx" => {
            companions.push((format!("{stem}.test.{ext}"), "test".into()));
            companions.push((format!("{stem}.spec.{ext}"), "spec".into()));
        }
        "py" => {
            companions.push((format!("test_{stem}.py"), "test".into()));
        }
        "go" => {
            companions.push((format!("{stem}_test.go"), "test".into()));
        }
        _ => {}
    }
    companions
}

/// Whether tests should run for this file type.
pub fn should_run_tests(changed: &Path) -> bool {
    let ext = changed.extension().and_then(|s| s.to_str()).unwrap_or("");
    matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "rb" | "java")
}

/// EC-15.2: Check if file was modified very recently (human likely editing).
pub fn is_file_recently_modified(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|mtime| mtime.elapsed().map(|d| d.as_secs() < 2).unwrap_or(false))
        .unwrap_or(false)
}

// ── Helpers ──

/// EC-15.5: Check if a file is a code file (not creative writing, docs, etc).
fn is_code_file(path: &str) -> bool {
    let ext = Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("");
    matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "rb" | "java" | "c" | "cpp" | "h" | "swift" | "kt")
}

/// Get the programming language from file extension.
fn file_language(path: &str) -> &str {
    let ext = Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("");
    match ext {
        "rs" => "rust", "ts" | "tsx" => "typescript", "js" | "jsx" => "javascript",
        "py" => "python", "go" => "go", "rb" => "ruby", "java" => "java",
        "c" | "cpp" | "h" => "c/c++", "swift" => "swift", "kt" => "kotlin",
        _ => "unknown",
    }
}

// ── Middleware ──

/// Collaboration middleware — injects file change context and suggestions.
pub struct CollabMiddleware {
    state: CollaborationState,
    observer: Option<hydra_desktop::FileObserver>,
}

impl CollabMiddleware {
    pub fn new() -> Self {
        // Auto-enable file observer for current working directory
        let observer = std::env::current_dir().ok().map(|dir| {
            hydra_desktop::FileObserver::new(&dir, 2000)
        });
        Self { state: CollaborationState::new(), observer }
    }
}

impl CycleMiddleware for CollabMiddleware {
    fn name(&self) -> &'static str { "collaboration" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        if self.state.mode == CollabMode::Off { return; }
        // Scan file observer if active
        if let Some(obs) = &mut self.observer {
            if !obs.is_debouncing() {
                let changes = obs.drain_changes();
                for change in &changes {
                    let path_str = change.path.to_string_lossy().to_string();
                    self.state.recent_changes.push(path_str.clone());
                    self.state.last_activity = Instant::now();
                    // Pair mode: queue tests for code files
                    if self.state.mode == CollabMode::PairProgramming && should_run_tests(&change.path) {
                        self.state.pending_tests.push(path_str);
                    }
                }
            }
        }
        // Shadow mode: check for idle and generate suggestions
        if self.state.mode == CollabMode::Shadow && detect_idle(&self.state) {
            let genome = hydra_genome::GenomeStore::open();
            if let Some(suggestion) = generate_suggestion(&self.state.recent_changes, &genome) {
                perceived.enrichments.insert("collab_suggestion".into(),
                    format!("[Suggestion for {}] {}", suggestion.source_file, suggestion.suggestion));
                self.state.suggestions.push(suggestion);
            }
        }
        // Pair mode: inject pending tests as enrichment
        if !self.state.pending_tests.is_empty() {
            perceived.enrichments.insert("collab_tests".into(),
                format!("Files needing tests: {}", self.state.pending_tests.join(", ")));
        }
    }

    fn enrich_prompt(&self, _perceived: &PerceivedInput) -> Vec<String> {
        if self.state.mode == CollabMode::Off { return Vec::new(); }
        let mut lines = vec![format!("[Collaboration: {} mode]", self.state.mode.label())];
        if !self.state.recent_changes.is_empty() {
            let recent: Vec<&String> = self.state.recent_changes.iter().rev().take(3).collect();
            lines.push(format!("  Recent changes: {}", recent.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }
        if !self.state.pending_tests.is_empty() {
            lines.push(format!("  Pending tests: {}", self.state.pending_tests.len()));
        }
        lines
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if self.state.mode == CollabMode::Off { return; }
        // Record accepted suggestions to genome
        if !self.state.suggestions.is_empty() && cycle.success {
            let mut genome = hydra_genome::GenomeStore::open();
            for suggestion in &self.state.suggestions {
                let desc = format!("collab suggestion {}", suggestion.source_file);
                let approach = hydra_genome::ApproachSignature::new(
                    "collaboration", vec![suggestion.suggestion.clone()], vec!["pair".into()]);
                if let Err(e) = genome.add_from_operation(&desc, approach, suggestion.confidence) {
                    eprintln!("hydra-collab: genome write failed: {e}");
                }
            }
            self.state.suggestions.clear();
        }
        // Clear pending tests after they've been communicated
        self.state.pending_tests.clear();
    }
}

// ── Public API ──

/// Get the current collaboration mode label.
pub fn current_mode() -> &'static str { "off" }

/// Summary of collaboration state.
pub fn status_summary() -> String {
    "Collaboration: off. Use /collab shadow or /collab pair <dir> to enable.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_detection() {
        let mut state = CollaborationState::new();
        state.idle_threshold_secs = 0; // Immediate idle for testing
        assert!(detect_idle(&state));
    }

    #[test]
    fn companion_file_mapping_rs() {
        let comps = identify_companion_files(Path::new("src/auth.rs"));
        assert!(!comps.is_empty());
        assert!(comps[0].0.contains("auth_test"));
    }

    #[test]
    fn companion_file_mapping_ts() {
        let comps = identify_companion_files(Path::new("src/auth.ts"));
        assert!(!comps.is_empty());
        assert!(comps[0].0.contains("auth.test.ts"));
    }

    #[test]
    fn should_run_tests_code_files() {
        assert!(should_run_tests(Path::new("foo.rs")));
        assert!(should_run_tests(Path::new("bar.py")));
        assert!(should_run_tests(Path::new("baz.go")));
        assert!(!should_run_tests(Path::new("readme.md")));
        assert!(!should_run_tests(Path::new("style.css")));
    }

    #[test]
    fn is_code_file_check() {
        assert!(is_code_file("main.rs"));
        assert!(is_code_file("app.tsx"));
        assert!(!is_code_file("notes.md"));
        assert!(!is_code_file("data.csv"));
    }

    #[test]
    fn mode_transitions() {
        let mut state = CollaborationState::new();
        assert_eq!(state.mode, CollabMode::Off);
        state.set_mode(CollabMode::Shadow, None);
        assert_eq!(state.mode, CollabMode::Shadow);
        state.set_mode(CollabMode::PairProgramming, Some(Path::new("/project")));
        assert_eq!(state.mode, CollabMode::PairProgramming);
        assert!(state.watch_dir.is_some());
    }

    #[test]
    fn middleware_name() {
        let mw = CollabMiddleware::new();
        assert_eq!(mw.name(), "collaboration");
    }
}
