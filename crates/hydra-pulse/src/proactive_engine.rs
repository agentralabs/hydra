//! Proactive file engine — analyzes filesystem changes and generates suggestions.
//!
//! Correlation rules detect common patterns (Cargo.toml edits, test file changes,
//! merge conflicts) and produce prioritized suggestions.

use std::collections::HashMap;
use std::path::Path;

use super::file_watcher::{ChangeKind, FileChange};

/// A suggestion generated from file change patterns.
#[derive(Debug, Clone)]
pub struct ProactiveSuggestion {
    pub title: String,
    pub message: String,
    pub priority: SuggestionPriority,
    pub action: Option<SuggestedAction>,
}

/// Priority level for a proactive suggestion.
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionPriority {
    /// Merge conflicts, build failures — needs immediate attention.
    Urgent,
    /// Cargo.toml changes, new test files — should act soon.
    Normal,
    /// General file changes — informational.
    Low,
}

/// A concrete action the user can take in response to a suggestion.
#[derive(Debug, Clone)]
pub enum SuggestedAction {
    /// Run a shell command (e.g., "cargo check").
    RunCommand(String),
    /// Open / review a file path.
    ReviewFile(String),
    /// Run tests for a specific crate (e.g., "cargo test -p hydra-pulse").
    RunTests(String),
}

/// Analyzes file changes and generates proactive suggestions.
pub struct ProactiveFileEngine {
    recent_changes: Vec<FileChange>,
    max_recent: usize,
}

impl ProactiveFileEngine {
    pub fn new() -> Self {
        Self {
            recent_changes: Vec::new(),
            max_recent: 500,
        }
    }

    /// Process new changes and generate suggestions.
    pub fn process_changes(&mut self, changes: &[FileChange]) -> Vec<ProactiveSuggestion> {
        // Append to history, trimming if over capacity
        self.recent_changes.extend_from_slice(changes);
        if self.recent_changes.len() > self.max_recent {
            let drain_count = self.recent_changes.len() - self.max_recent;
            self.recent_changes.drain(..drain_count);
        }

        let mut suggestions = Vec::new();
        let mut crate_change_counts: HashMap<String, usize> = HashMap::new();

        for change in changes {
            let path_str = change.path.to_string_lossy();

            // Rule 1: Cargo.toml modified → suggest cargo check
            if is_cargo_toml(&change.path) && change.kind == ChangeKind::Modified {
                suggestions.push(ProactiveSuggestion {
                    title: "Cargo.toml changed".into(),
                    message: format!("Dependency file modified: {}", path_str),
                    priority: SuggestionPriority::Normal,
                    action: Some(SuggestedAction::RunCommand("cargo check -j 1".into())),
                });
            }

            // Rule 2: Merge conflict markers
            if is_merge_conflict_file(&change.path) {
                suggestions.push(ProactiveSuggestion {
                    title: "Possible merge conflict".into(),
                    message: format!("Conflict marker file detected: {}", path_str),
                    priority: SuggestionPriority::Urgent,
                    action: Some(SuggestedAction::ReviewFile(path_str.to_string())),
                });
            }

            // Rule 3: Test file changed → suggest run tests
            if is_test_file(&change.path) {
                let crate_name = extract_crate_name(&change.path);
                let cmd = match &crate_name {
                    Some(name) => format!("cargo test -p {name} -j 1"),
                    None => "cargo test -j 1".into(),
                };
                suggestions.push(ProactiveSuggestion {
                    title: "Test file changed".into(),
                    message: format!("Test modified: {}", path_str),
                    priority: SuggestionPriority::Normal,
                    action: Some(SuggestedAction::RunTests(cmd)),
                });
            }

            // Rule 4: Lock file changed
            if is_lock_file(&change.path) {
                suggestions.push(ProactiveSuggestion {
                    title: "Lock file updated".into(),
                    message: "Cargo.lock changed — dependency tree may have shifted".into(),
                    priority: SuggestionPriority::Normal,
                    action: Some(SuggestedAction::RunCommand("cargo check -j 1".into())),
                });
            }

            // Rule 6: New .rs file created
            if change.kind == ChangeKind::Created && is_rust_file(&change.path) {
                suggestions.push(ProactiveSuggestion {
                    title: "New Rust file".into(),
                    message: format!("Created: {}", path_str),
                    priority: SuggestionPriority::Low,
                    action: Some(SuggestedAction::RunCommand("cargo check -j 1".into())),
                });
            }

            // Rule 7: Schema file changed
            if is_schema_file(&change.path) {
                let crate_name = extract_crate_name(&change.path);
                let cmd = match &crate_name {
                    Some(name) => format!("cargo test -p {name} -j 1"),
                    None => "cargo test -j 1".into(),
                };
                suggestions.push(ProactiveSuggestion {
                    title: "Schema file changed".into(),
                    message: format!("Database schema modified: {}", path_str),
                    priority: SuggestionPriority::Normal,
                    action: Some(SuggestedAction::RunTests(cmd)),
                });
            }

            // Track per-crate changes for Rule 5
            if is_rust_file(&change.path) {
                if let Some(name) = extract_crate_name(&change.path) {
                    *crate_change_counts.entry(name).or_insert(0) += 1;
                }
            }
        }

        // Rule 5: Multiple files in the same crate changed
        for (crate_name, count) in &crate_change_counts {
            if *count >= 3 {
                suggestions.push(ProactiveSuggestion {
                    title: format!("Multiple changes in {crate_name}"),
                    message: format!("{count} files changed in {crate_name}"),
                    priority: SuggestionPriority::Low,
                    action: Some(SuggestedAction::RunCommand(format!(
                        "cargo check -p {crate_name} -j 1"
                    ))),
                });
            }
        }

        suggestions
    }

    /// Clear all change history.
    pub fn clear(&mut self) {
        self.recent_changes.clear();
    }

    /// Number of changes currently tracked.
    pub fn tracked_count(&self) -> usize {
        self.recent_changes.len()
    }
}

impl Default for ProactiveFileEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn is_cargo_toml(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n == "Cargo.toml")
}

fn is_lock_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n == "Cargo.lock")
}

fn is_rust_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("rs")
}

fn is_test_file(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.ends_with("_test.rs") || s.ends_with("_tests.rs") || s.contains("/tests/")
}

fn is_schema_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.starts_with("schema") && n.ends_with(".rs"))
}

fn is_merge_conflict_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map_or(false, |e| e == "orig")
}

/// Extract the crate name from a path like `crates/<name>/src/foo.rs`.
fn extract_crate_name(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();
    for (i, c) in components.iter().enumerate() {
        if let std::path::Component::Normal(os) = c {
            if os.to_str() == Some("crates") {
                if let Some(std::path::Component::Normal(name)) = components.get(i + 1) {
                    return name.to_str().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    fn make_change(path: &str, kind: ChangeKind) -> FileChange {
        FileChange {
            path: PathBuf::from(path),
            kind,
            timestamp: Instant::now(),
        }
    }

    #[test]
    fn test_cargo_toml_suggestion() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change("crates/hydra-pulse/Cargo.toml", ChangeKind::Modified)];
        let suggestions = engine.process_changes(&changes);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.title == "Cargo.toml changed"));
    }

    #[test]
    fn test_test_file_suggestion() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change(
            "crates/hydra-pulse/src/pulse_tests.rs",
            ChangeKind::Modified,
        )];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions.iter().any(|s| s.title == "Test file changed"));
    }

    #[test]
    fn test_lock_file_suggestion() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change("Cargo.lock", ChangeKind::Modified)];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions.iter().any(|s| s.title == "Lock file updated"));
    }

    #[test]
    fn test_merge_conflict_file() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change("src/main.rs.orig", ChangeKind::Created)];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions
            .iter()
            .any(|s| s.priority == SuggestionPriority::Urgent));
    }

    #[test]
    fn test_new_rs_file() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change("crates/hydra-pulse/src/new_mod.rs", ChangeKind::Created)];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions.iter().any(|s| s.title == "New Rust file"));
    }

    #[test]
    fn test_schema_file_suggestion() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change(
            "crates/hydra-db/src/schema.rs",
            ChangeKind::Modified,
        )];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions.iter().any(|s| s.title == "Schema file changed"));
    }

    #[test]
    fn test_multi_file_crate_suggestion() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![
            make_change("crates/hydra-pulse/src/a.rs", ChangeKind::Modified),
            make_change("crates/hydra-pulse/src/b.rs", ChangeKind::Modified),
            make_change("crates/hydra-pulse/src/c.rs", ChangeKind::Modified),
        ];
        let suggestions = engine.process_changes(&changes);
        assert!(suggestions
            .iter()
            .any(|s| s.title.contains("Multiple changes")));
    }

    #[test]
    fn test_extract_crate_name() {
        assert_eq!(
            extract_crate_name(Path::new("crates/hydra-pulse/src/lib.rs")),
            Some("hydra-pulse".into())
        );
        assert_eq!(extract_crate_name(Path::new("src/main.rs")), None);
    }

    #[test]
    fn test_clear() {
        let mut engine = ProactiveFileEngine::new();
        let changes = vec![make_change("foo.rs", ChangeKind::Modified)];
        engine.process_changes(&changes);
        assert!(engine.tracked_count() > 0);
        engine.clear();
        assert_eq!(engine.tracked_count(), 0);
    }
}
