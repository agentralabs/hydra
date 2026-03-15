//! Global change tracker — records all file modifications with undo support.
//!
//! Provides session-level undo for any file changes made by the cognitive loop.
//! Uses `OnceLock<Mutex<...>>` for safe global access from async contexts.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Global change tracker instance.
static GLOBAL_CHANGE_TRACKER: OnceLock<Mutex<ChangeTracker>> = OnceLock::new();

/// Get or initialize the global change tracker.
pub fn global_tracker() -> &'static Mutex<ChangeTracker> {
    GLOBAL_CHANGE_TRACKER.get_or_init(|| {
        Mutex::new(ChangeTracker::new("default"))
    })
}

/// Reset the global tracker with a new session ID.
pub fn reset_global_tracker(session_id: &str) {
    if let Ok(mut tracker) = global_tracker().lock() {
        tracker.changes.clear();
        tracker.session_id = session_id.to_string();
    }
}

/// Type of file change recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Created,
    Edited,
    Deleted,
}

impl std::fmt::Display for FileChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileChangeType::Created => write!(f, "Created"),
            FileChangeType::Edited => write!(f, "Edited"),
            FileChangeType::Deleted => write!(f, "Deleted"),
        }
    }
}

/// A recorded file change with full content for undo.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub file_path: PathBuf,
    pub change_type: FileChangeType,
    pub timestamp: String,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub diff_summary: String,
    pub description: String,
}

/// Tracks all file changes in a session with undo capabilities.
#[derive(Debug)]
pub struct ChangeTracker {
    pub changes: Vec<FileChange>,
    pub session_id: String,
}

impl ChangeTracker {
    /// Create a new change tracker for a session.
    pub fn new(session_id: &str) -> Self {
        Self {
            changes: Vec::new(),
            session_id: session_id.to_string(),
        }
    }

    /// Record a file change.
    pub fn record_change(&mut self, change: FileChange) {
        eprintln!(
            "[hydra:tracker] Recorded {} on {} — {}",
            change.change_type,
            change.file_path.display(),
            change.description,
        );
        self.changes.push(change);
    }

    /// Undo the last recorded change by restoring old content.
    ///
    /// For Created files, the file is deleted.
    /// For Edited files, old_content is restored.
    /// For Deleted files, old_content is written back.
    pub fn undo_last(&mut self) -> Result<FileChange, String> {
        let change = self.changes.pop().ok_or_else(|| {
            "No changes to undo".to_string()
        })?;
        restore_change(&change)?;
        eprintln!(
            "[hydra:tracker] Undid {} on {}",
            change.change_type,
            change.file_path.display(),
        );
        Ok(change)
    }

    /// Undo all changes to a specific file, most recent first.
    ///
    /// Returns all reverted changes.
    pub fn undo_file(&mut self, path: &Path) -> Result<Vec<FileChange>, String> {
        let mut reverted = Vec::new();
        // Collect indices of changes to this file (reverse order)
        let indices: Vec<usize> = self.changes
            .iter()
            .enumerate()
            .filter(|(_, c)| c.file_path == path)
            .map(|(i, _)| i)
            .rev()
            .collect();

        if indices.is_empty() {
            return Err(format!("No changes recorded for {}", path.display()));
        }

        // Remove and restore in reverse order
        for idx in &indices {
            let change = self.changes.remove(*idx);
            restore_change(&change)?;
            reverted.push(change);
        }

        eprintln!(
            "[hydra:tracker] Reverted {} changes to {}",
            reverted.len(),
            path.display(),
        );
        Ok(reverted)
    }

    /// Generate a formatted summary of all changes in the session.
    pub fn session_summary(&self) -> String {
        if self.changes.is_empty() {
            return format!(
                "Session {}: No file changes recorded.",
                self.session_id
            );
        }

        let mut out = format!(
            "Session {} — {} change(s):\n",
            self.session_id,
            self.changes.len(),
        );
        out.push_str(&format!(
            "{:<8} {:<50} {:<10} {}\n",
            "Type", "File", "Diff", "Description"
        ));
        out.push_str(&"-".repeat(90));
        out.push('\n');

        for change in &self.changes {
            let file_display = change
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            out.push_str(&format!(
                "{:<8} {:<50} {:<10} {}\n",
                change.change_type,
                file_display,
                change.diff_summary,
                change.description,
            ));
        }
        out
    }

    /// Count of changes recorded.
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
}

/// Restore a file to its pre-change state.
fn restore_change(change: &FileChange) -> Result<(), String> {
    match change.change_type {
        FileChangeType::Created => {
            // Undo creation = delete the file
            std::fs::remove_file(&change.file_path).map_err(|e| {
                format!(
                    "Failed to remove created file {}: {}",
                    change.file_path.display(),
                    e
                )
            })?;
        }
        FileChangeType::Edited => {
            // Undo edit = restore old content
            let old = change.old_content.as_ref().ok_or_else(|| {
                "No old_content stored for edit undo".to_string()
            })?;
            std::fs::write(&change.file_path, old).map_err(|e| {
                format!(
                    "Failed to restore {}: {}",
                    change.file_path.display(),
                    e
                )
            })?;
        }
        FileChangeType::Deleted => {
            // Undo deletion = recreate with old content
            let old = change.old_content.as_ref().ok_or_else(|| {
                "No old_content stored for delete undo".to_string()
            })?;
            std::fs::write(&change.file_path, old).map_err(|e| {
                format!(
                    "Failed to recreate {}: {}",
                    change.file_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn make_change(
        path: &Path,
        ctype: FileChangeType,
        old: Option<&str>,
        new: Option<&str>,
    ) -> FileChange {
        FileChange {
            file_path: path.to_path_buf(),
            change_type: ctype,
            timestamp: "2026-03-15T00:00:00Z".into(),
            old_content: old.map(|s| s.to_string()),
            new_content: new.map(|s| s.to_string()),
            diff_summary: "+1 -0".into(),
            description: "test change".into(),
        }
    }

    #[test]
    fn test_record_and_count() {
        let mut tracker = ChangeTracker::new("test-session");
        assert_eq!(tracker.change_count(), 0);
        let f = temp_file("content");
        tracker.record_change(make_change(
            f.path(),
            FileChangeType::Edited,
            Some("old"),
            Some("new"),
        ));
        assert_eq!(tracker.change_count(), 1);
    }

    #[test]
    fn test_undo_last_edit() {
        let f = temp_file("original");
        // Simulate an edit: write new content first
        std::fs::write(f.path(), "modified").unwrap();

        let mut tracker = ChangeTracker::new("test");
        tracker.record_change(make_change(
            f.path(),
            FileChangeType::Edited,
            Some("original"),
            Some("modified"),
        ));

        let undone = tracker.undo_last().unwrap();
        assert_eq!(undone.change_type, FileChangeType::Edited);
        let restored = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(restored, "original");
        assert_eq!(tracker.change_count(), 0);
    }

    #[test]
    fn test_undo_last_creation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_file.txt");
        std::fs::write(&path, "created content").unwrap();

        let mut tracker = ChangeTracker::new("test");
        tracker.record_change(make_change(
            &path,
            FileChangeType::Created,
            None,
            Some("created content"),
        ));

        tracker.undo_last().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_undo_last_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deleted_file.txt");
        // File was deleted, so it doesn't exist on disk

        let mut tracker = ChangeTracker::new("test");
        tracker.record_change(make_change(
            &path,
            FileChangeType::Deleted,
            Some("old content"),
            None,
        ));

        tracker.undo_last().unwrap();
        let restored = std::fs::read_to_string(&path).unwrap();
        assert_eq!(restored, "old content");
    }

    #[test]
    fn test_undo_empty_fails() {
        let mut tracker = ChangeTracker::new("test");
        let err = tracker.undo_last().unwrap_err();
        assert!(err.contains("No changes"));
    }

    #[test]
    fn test_undo_file() {
        let f = temp_file("v1");
        // Simulate two edits
        std::fs::write(f.path(), "v3").unwrap();

        let mut tracker = ChangeTracker::new("test");
        tracker.record_change(make_change(
            f.path(),
            FileChangeType::Edited,
            Some("v1"),
            Some("v2"),
        ));
        tracker.record_change(make_change(
            f.path(),
            FileChangeType::Edited,
            Some("v2"),
            Some("v3"),
        ));

        let reverted = tracker.undo_file(f.path()).unwrap();
        assert_eq!(reverted.len(), 2);
        assert_eq!(tracker.change_count(), 0);
    }

    #[test]
    fn test_undo_file_not_found() {
        let mut tracker = ChangeTracker::new("test");
        let err = tracker.undo_file(Path::new("/no/such/file.rs")).unwrap_err();
        assert!(err.contains("No changes recorded"));
    }

    #[test]
    fn test_session_summary() {
        let tracker = ChangeTracker::new("s1");
        assert!(tracker.session_summary().contains("No file changes"));
        let mut tracker2 = ChangeTracker::new("s2");
        let f = temp_file("x");
        tracker2.record_change(make_change(f.path(), FileChangeType::Edited, Some("old"), Some("new")));
        let summary = tracker2.session_summary();
        assert!(summary.contains("1 change(s)"));
        assert!(summary.contains("Edited"));
    }

    #[test]
    fn test_global_tracker() {
        let tracker = global_tracker();
        assert!(tracker.lock().unwrap().change_count() < 10000);
        reset_global_tracker("fresh-session");
        let lock = tracker.lock().unwrap();
        assert_eq!(lock.session_id, "fresh-session");
        assert_eq!(lock.change_count(), 0);
    }
}
