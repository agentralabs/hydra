use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A persisted record of an undo action (metadata only, no closures)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoHistoryEntry {
    /// Unique identifier for the action
    pub action_id: String,
    /// Human-readable description
    pub description: String,
    /// When the action was performed
    pub performed_at: DateTime<Utc>,
    /// Whether this action has been undone
    pub undone: bool,
}

/// Append-only log of undo action metadata, suitable for persistence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UndoHistoryLog {
    entries: Vec<UndoHistoryEntry>,
}

impl UndoHistoryLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an action being performed
    pub fn record(&mut self, action_id: impl Into<String>, description: impl Into<String>) {
        self.entries.push(UndoHistoryEntry {
            action_id: action_id.into(),
            description: description.into(),
            performed_at: Utc::now(),
            undone: false,
        });
    }

    /// Mark an action as undone
    pub fn mark_undone(&mut self, action_id: &str) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.action_id == action_id) {
            entry.undone = true;
        }
    }

    /// Mark an action as redone
    pub fn mark_redone(&mut self, action_id: &str) {
        if let Some(entry) = self.entries.iter_mut().rev().find(|e| e.action_id == action_id) {
            entry.undone = false;
        }
    }

    /// Get all entries (newest last)
    pub fn entries(&self) -> &[UndoHistoryEntry] {
        &self.entries
    }

    /// Get the most recent N entries
    pub fn recent(&self, n: usize) -> &[UndoHistoryEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    /// Serialize to JSON for persistence
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_history_empty() {
        let log = UndoHistoryLog::new();
        assert!(log.entries().is_empty());
    }

    #[test]
    fn test_record_entry() {
        let mut log = UndoHistoryLog::new();
        log.record("a1", "Action 1");
        assert_eq!(log.entries().len(), 1);
        assert_eq!(log.entries()[0].action_id, "a1");
        assert_eq!(log.entries()[0].description, "Action 1");
        assert!(!log.entries()[0].undone);
    }

    #[test]
    fn test_mark_undone() {
        let mut log = UndoHistoryLog::new();
        log.record("a1", "Action 1");
        log.mark_undone("a1");
        assert!(log.entries()[0].undone);
    }

    #[test]
    fn test_mark_redone() {
        let mut log = UndoHistoryLog::new();
        log.record("a1", "Action 1");
        log.mark_undone("a1");
        log.mark_redone("a1");
        assert!(!log.entries()[0].undone);
    }

    #[test]
    fn test_mark_undone_nonexistent() {
        let mut log = UndoHistoryLog::new();
        log.mark_undone("nonexistent"); // Should not panic
    }

    #[test]
    fn test_recent() {
        let mut log = UndoHistoryLog::new();
        for i in 0..10 {
            log.record(format!("a{}", i), format!("Action {}", i));
        }
        let recent = log.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].action_id, "a7");
    }

    #[test]
    fn test_recent_more_than_available() {
        let mut log = UndoHistoryLog::new();
        log.record("a1", "Action 1");
        let recent = log.recent(100);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_json_roundtrip() {
        let mut log = UndoHistoryLog::new();
        log.record("a1", "Action 1");
        log.record("a2", "Action 2");
        log.mark_undone("a1");
        let json = log.to_json().unwrap();
        let restored = UndoHistoryLog::from_json(&json).unwrap();
        assert_eq!(restored.entries().len(), 2);
        assert!(restored.entries()[0].undone);
        assert!(!restored.entries()[1].undone);
    }

    #[test]
    fn test_default() {
        let log = UndoHistoryLog::default();
        assert!(log.entries().is_empty());
    }
}
