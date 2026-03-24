//! O15 File Observer — poll-based filesystem watcher for pair programming.
//! No `notify` dependency — uses std::fs::metadata timestamps.
//! EC-15.1: Debounce prevents auto-save spam (2s default).

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

/// Kind of filesystem change detected.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
}

impl ChangeKind {
    pub fn label(&self) -> &'static str {
        match self { Self::Created => "created", Self::Modified => "modified", Self::Deleted => "deleted" }
    }
}

/// A detected file change.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub timestamp: DateTime<Utc>,
}

/// Default ignored directory patterns.
const DEFAULT_IGNORES: &[&str] = &[
    "node_modules", ".git", "target", "__pycache__", ".next",
    "dist", "build", ".cache", ".vscode", ".idea",
];

/// Poll-based file observer for a directory.
pub struct FileObserver {
    watch_dir: PathBuf,
    file_states: HashMap<PathBuf, SystemTime>,
    /// EC-15.1: debounce window in milliseconds.
    debounce_ms: u64,
    last_change: Option<Instant>,
    pending_changes: Vec<FileChange>,
    ignored_patterns: Vec<String>,
}

impl FileObserver {
    pub fn new(dir: &Path, debounce_ms: u64) -> Self {
        let mut ignored: Vec<String> = DEFAULT_IGNORES.iter().map(|s| s.to_string()).collect();
        ignored.sort();
        Self {
            watch_dir: dir.to_path_buf(),
            file_states: HashMap::new(),
            debounce_ms,
            last_change: None,
            pending_changes: Vec::new(),
            ignored_patterns: ignored,
        }
    }

    /// Scan the directory for changes since last scan.
    pub fn scan(&mut self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        let mut current_files = HashMap::new();

        // Walk directory (shallow — first 2 levels to avoid deep traversal)
        if let Ok(entries) = std::fs::read_dir(&self.watch_dir) {
            for entry in entries.flatten() {
                self.scan_entry(&entry.path(), &mut current_files, &mut changes, 0);
            }
        }

        // Detect deletions: files in old state but not current
        for (path, _) in &self.file_states {
            if !current_files.contains_key(path) {
                changes.push(FileChange { path: path.clone(), kind: ChangeKind::Deleted, timestamp: Utc::now() });
            }
        }

        self.file_states = current_files;
        if !changes.is_empty() {
            self.last_change = Some(Instant::now());
            self.pending_changes.extend(changes.clone());
        }
        changes
    }

    fn scan_entry(&self, path: &Path, current: &mut HashMap<PathBuf, SystemTime>, changes: &mut Vec<FileChange>, depth: u8) {
        if depth > 2 { return; } // Max depth
        if self.is_ignored(path) { return; }

        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    self.scan_entry(&entry.path(), current, changes, depth + 1);
                }
            }
            return;
        }

        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                current.insert(path.to_path_buf(), mtime);
                match self.file_states.get(path) {
                    None => changes.push(FileChange { path: path.to_path_buf(), kind: ChangeKind::Created, timestamp: Utc::now() }),
                    Some(old_mtime) if *old_mtime != mtime => {
                        changes.push(FileChange { path: path.to_path_buf(), kind: ChangeKind::Modified, timestamp: Utc::now() });
                    }
                    _ => {}
                }
            }
        }
    }

    /// EC-15.1: Whether within the debounce window.
    pub fn is_debouncing(&self) -> bool {
        self.last_change.map(|t| t.elapsed().as_millis() < self.debounce_ms as u128).unwrap_or(false)
    }

    /// Drain pending changes (only after debounce window).
    pub fn drain_changes(&mut self) -> Vec<FileChange> {
        if self.is_debouncing() { return Vec::new(); }
        std::mem::take(&mut self.pending_changes)
    }

    /// Check if a path matches an ignored pattern.
    fn is_ignored(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.ignored_patterns.iter().any(|p| path_str.contains(p))
    }

    /// Add an ignore pattern.
    pub fn add_ignore(&mut self, pattern: &str) {
        self.ignored_patterns.push(pattern.to_string());
    }

    pub fn watch_dir(&self) -> &Path { &self.watch_dir }
    pub fn file_count(&self) -> usize { self.file_states.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn creates_with_defaults() {
        let obs = FileObserver::new(&PathBuf::from("/tmp"), 2000);
        assert_eq!(obs.debounce_ms, 2000);
        assert!(!obs.ignored_patterns.is_empty());
    }

    #[test]
    fn ignores_node_modules() {
        let obs = FileObserver::new(&PathBuf::from("/tmp"), 2000);
        assert!(obs.is_ignored(Path::new("/project/node_modules/foo.js")));
        assert!(obs.is_ignored(Path::new("/project/.git/HEAD")));
        assert!(obs.is_ignored(Path::new("/project/target/debug/binary")));
        assert!(!obs.is_ignored(Path::new("/project/src/main.rs")));
    }

    #[test]
    fn change_kind_labels() {
        assert_eq!(ChangeKind::Created.label(), "created");
        assert_eq!(ChangeKind::Modified.label(), "modified");
        assert_eq!(ChangeKind::Deleted.label(), "deleted");
    }

    #[test]
    fn not_debouncing_initially() {
        let obs = FileObserver::new(&PathBuf::from("/tmp"), 2000);
        assert!(!obs.is_debouncing());
    }
}
