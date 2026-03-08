use serde::{Deserialize, Serialize};

/// Summary generated at task completion, suitable for multiple output formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSummary {
    pub headline: String,
    pub actions: Vec<String>,
    pub changes: Vec<ChangeSummary>,
    pub stats: CompletionStats,
    pub next_steps: Vec<String>,
    pub warnings: Vec<String>,
}

/// Description of a single file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub path: String,
    pub change_type: ChangeType,
    pub lines_added: u32,
    pub lines_removed: u32,
}

/// Type of file change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Statistics about the completed operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionStats {
    pub duration_ms: u64,
    pub files_affected: usize,
    pub tokens_used: u64,
    pub phases_completed: usize,
}

impl CompletionSummary {
    /// Create a new completion summary with the given headline
    pub fn new(headline: &str) -> Self {
        Self {
            headline: headline.to_string(),
            actions: Vec::new(),
            changes: Vec::new(),
            stats: CompletionStats::default(),
            next_steps: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an action that was performed
    pub fn add_action(&mut self, action: &str) {
        self.actions.push(action.to_string());
    }

    /// Add a file change record
    pub fn add_change(&mut self, change: ChangeSummary) {
        self.stats.files_affected = self.changes.len() + 1;
        self.changes.push(change);
    }

    /// Add a suggested next step
    pub fn add_next_step(&mut self, step: &str) {
        self.next_steps.push(step.to_string());
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }

    /// Format for CLI display
    pub fn format_cli(&self) -> String {
        let mut out = String::new();

        // Headline
        out.push_str(&format!("Done: {}\n", self.headline));

        // Actions
        if !self.actions.is_empty() {
            for action in &self.actions {
                out.push_str(&format!("  - {}\n", action));
            }
        }

        // Changes
        if !self.changes.is_empty() {
            out.push('\n');
            out.push_str("Changes:\n");
            for change in &self.changes {
                let symbol = match change.change_type {
                    ChangeType::Created => "+",
                    ChangeType::Modified => "~",
                    ChangeType::Deleted => "-",
                    ChangeType::Renamed => ">",
                };
                out.push_str(&format!(
                    "  {} {} (+{} -{} lines)\n",
                    symbol, change.path, change.lines_added, change.lines_removed
                ));
            }
        }

        // Stats
        out.push_str(&format!(
            "\n{} files affected, {}ms elapsed, {} tokens used\n",
            self.stats.files_affected, self.stats.duration_ms, self.stats.tokens_used
        ));

        // Warnings
        for warning in &self.warnings {
            out.push_str(&format!("  WARNING: {}\n", warning));
        }

        // Next steps
        if !self.next_steps.is_empty() {
            out.push_str("\nNext steps:\n");
            for step in &self.next_steps {
                out.push_str(&format!("  - {}\n", step));
            }
        }

        out
    }

    /// Format for voice output (concise, natural language)
    pub fn format_voice(&self) -> String {
        let file_word = if self.stats.files_affected == 1 {
            "file"
        } else {
            "files"
        };
        let change_count = self.changes.len();
        let change_word = if change_count == 1 {
            "change"
        } else {
            "changes"
        };

        let mut voice = format!(
            "Done! {}. I made {} {} to {} {}.",
            self.headline, change_count, change_word, self.stats.files_affected, file_word
        );

        if !self.next_steps.is_empty() {
            voice.push_str(&format!(" Next up: {}.", self.next_steps[0]));
        }

        voice
    }

    /// Format as JSON string
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get the total lines added across all changes
    pub fn total_lines_added(&self) -> u32 {
        self.changes.iter().map(|c| c.lines_added).sum()
    }

    /// Get the total lines removed across all changes
    pub fn total_lines_removed(&self) -> u32 {
        self.changes.iter().map(|c| c.lines_removed).sum()
    }
}

impl ChangeSummary {
    pub fn new(path: &str, change_type: ChangeType, lines_added: u32, lines_removed: u32) -> Self {
        Self {
            path: path.to_string(),
            change_type,
            lines_added,
            lines_removed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_summary() {
        let summary = CompletionSummary::new("Task completed");
        assert_eq!(summary.headline, "Task completed");
        assert!(summary.actions.is_empty());
        assert!(summary.changes.is_empty());
        assert!(summary.next_steps.is_empty());
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn test_add_action() {
        let mut summary = CompletionSummary::new("Test");
        summary.add_action("Created file");
        summary.add_action("Ran tests");
        assert_eq!(summary.actions.len(), 2);
    }

    #[test]
    fn test_add_change() {
        let mut summary = CompletionSummary::new("Test");
        summary.add_change(ChangeSummary::new("src/main.rs", ChangeType::Modified, 10, 5));
        assert_eq!(summary.changes.len(), 1);
        assert_eq!(summary.stats.files_affected, 1);
    }

    #[test]
    fn test_total_lines() {
        let mut summary = CompletionSummary::new("Test");
        summary.add_change(ChangeSummary::new("a.rs", ChangeType::Created, 10, 0));
        summary.add_change(ChangeSummary::new("b.rs", ChangeType::Modified, 5, 3));
        assert_eq!(summary.total_lines_added(), 15);
        assert_eq!(summary.total_lines_removed(), 3);
    }

    #[test]
    fn test_format_cli() {
        let mut summary = CompletionSummary::new("Refactored module");
        summary.add_action("Split into 3 files");
        summary.add_change(ChangeSummary::new("src/lib.rs", ChangeType::Modified, 5, 20));
        summary.add_change(ChangeSummary::new("src/new.rs", ChangeType::Created, 15, 0));
        summary.add_warning("Unused import in new.rs");
        summary.add_next_step("Run cargo test");
        let cli = summary.format_cli();
        assert!(cli.contains("Refactored module"));
        assert!(cli.contains("Split into 3 files"));
        assert!(cli.contains("src/lib.rs"));
        assert!(cli.contains("WARNING"));
        assert!(cli.contains("cargo test"));
    }

    #[test]
    fn test_format_voice() {
        let mut summary = CompletionSummary::new("Updated config");
        summary.add_change(ChangeSummary::new("config.toml", ChangeType::Modified, 1, 1));
        summary.add_next_step("Restart the server");
        let voice = summary.format_voice();
        assert!(voice.contains("Done!"));
        assert!(voice.contains("1 change"));
        assert!(voice.contains("1 file"));
        assert!(voice.contains("Restart the server"));
    }

    #[test]
    fn test_format_voice_plural() {
        let mut summary = CompletionSummary::new("Multi-file update");
        summary.add_change(ChangeSummary::new("a.rs", ChangeType::Modified, 1, 0));
        summary.add_change(ChangeSummary::new("b.rs", ChangeType::Modified, 1, 0));
        let voice = summary.format_voice();
        assert!(voice.contains("2 changes"));
        assert!(voice.contains("2 files"));
    }

    #[test]
    fn test_format_json() {
        let summary = CompletionSummary::new("Test");
        let json = summary.format_json();
        assert!(json.contains("Test"));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["headline"], "Test");
    }

    #[test]
    fn test_change_type_serde() {
        for ct in [ChangeType::Created, ChangeType::Modified, ChangeType::Deleted, ChangeType::Renamed] {
            let json = serde_json::to_string(&ct).unwrap();
            let restored: ChangeType = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, ct);
        }
    }

    #[test]
    fn test_change_summary_new() {
        let cs = ChangeSummary::new("test.rs", ChangeType::Created, 100, 0);
        assert_eq!(cs.path, "test.rs");
        assert_eq!(cs.change_type, ChangeType::Created);
        assert_eq!(cs.lines_added, 100);
        assert_eq!(cs.lines_removed, 0);
    }

    #[test]
    fn test_completion_stats_default() {
        let stats = CompletionStats::default();
        assert_eq!(stats.duration_ms, 0);
        assert_eq!(stats.files_affected, 0);
        assert_eq!(stats.tokens_used, 0);
        assert_eq!(stats.phases_completed, 0);
    }
}
