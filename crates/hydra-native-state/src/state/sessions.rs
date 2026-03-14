//! Session management — persisted conversation sessions.
//!
//! Sessions are stored as JSON files in `~/.hydra/sessions/{id}.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Status of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Error,
}

/// A single conversation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: Vec<SessionMessage>,
    pub created_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
}

impl Session {
    /// Create a new session with the given ID and first user message as title.
    pub fn new(id: &str, first_message: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let title = if first_message.len() > 60 {
            format!("{}...", &first_message[..57])
        } else {
            first_message.to_string()
        };
        Self {
            id: id.to_string(),
            title,
            messages: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            status: SessionStatus::Active,
        }
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, role: &str, content: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        self.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now.clone(),
        });
        self.updated_at = now;
    }

    /// Mark session as completed.
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Mark session as errored.
    pub fn set_error(&mut self) {
        self.status = SessionStatus::Error;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

/// Manages all sessions with persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub sessions: HashMap<String, Session>,
    pub active_session_id: Option<String>,
    #[serde(skip)]
    next_counter: u32,
}

impl SessionStore {
    /// Create an empty session store.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_session_id: None,
            next_counter: 0,
        }
    }

    /// Path to the sessions directory.
    fn sessions_dir() -> Option<PathBuf> {
        dirs_or_home().map(|h| h.join(".hydra").join("sessions"))
    }

    /// Create a new session and set it as active.
    pub fn new_session(&mut self, first_message: &str) -> String {
        self.next_counter += 1;
        let id = format!("session-{}-{}", chrono::Utc::now().timestamp(), self.next_counter);
        let session = Session::new(&id, first_message);
        self.sessions.insert(id.clone(), session);
        self.active_session_id = Some(id.clone());
        self.save_session(&id);
        id
    }

    /// Add a message to the active session.
    pub fn add_message(&mut self, role: &str, content: &str) {
        if let Some(ref id) = self.active_session_id.clone() {
            if let Some(session) = self.sessions.get_mut(id) {
                session.add_message(role, content);
                self.save_session(id);
            }
        }
    }

    /// Get the active session.
    pub fn active_session(&self) -> Option<&Session> {
        self.active_session_id.as_ref().and_then(|id| self.sessions.get(id))
    }

    /// Switch to a different session.
    pub fn switch_to(&mut self, session_id: &str) {
        if self.sessions.contains_key(session_id) {
            self.active_session_id = Some(session_id.to_string());
        }
    }

    /// Get all sessions sorted by updated_at (most recent first).
    pub fn sorted_sessions(&self) -> Vec<&Session> {
        let mut sessions: Vec<&Session> = self.sessions.values().collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    /// Get sessions filtered by status.
    pub fn sessions_by_status(&self, status: SessionStatus) -> Vec<&Session> {
        self.sorted_sessions()
            .into_iter()
            .filter(|s| s.status == status)
            .collect()
    }

    /// Complete the active session.
    pub fn complete_active(&mut self) {
        if let Some(ref id) = self.active_session_id.clone() {
            if let Some(session) = self.sessions.get_mut(id) {
                session.complete();
                self.save_session(id);
            }
        }
    }

    /// Save a single session to disk.
    fn save_session(&self, id: &str) {
        if let Some(dir) = Self::sessions_dir() {
            let _ = std::fs::create_dir_all(&dir);
            if let Some(session) = self.sessions.get(id) {
                let path = dir.join(format!("{}.json", id));
                if let Ok(json) = serde_json::to_string_pretty(session) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    /// Load all sessions from disk.
    pub fn load_all() -> Self {
        let mut store = Self::new();
        if let Some(dir) = Self::sessions_dir() {
            if dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("json") {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                                    store.sessions.insert(session.id.clone(), session);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Set counter past existing sessions
        store.next_counter = store.sessions.len() as u32;
        store
    }

    /// Delete a session from memory and disk.
    pub fn delete_session(&mut self, id: &str) {
        self.sessions.remove(id);
        if self.active_session_id.as_deref() == Some(id) {
            self.active_session_id = None;
        }
        if let Some(dir) = Self::sessions_dir() {
            let path = dir.join(format!("{}.json", id));
            let _ = std::fs::remove_file(path);
        }
    }

    /// Get session count.
    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Get home directory path (cross-platform).
fn dirs_or_home() -> Option<PathBuf> {
    Some(PathBuf::from(crate::utils::home_dir()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-1", "Build a REST API with auth");
        assert_eq!(session.id, "test-1");
        assert_eq!(session.title, "Build a REST API with auth");
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_session_title_truncation() {
        let long_msg = "a".repeat(100);
        let session = Session::new("test-2", &long_msg);
        assert!(session.title.len() <= 63); // 57 + "..."
        assert!(session.title.ends_with("..."));
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new("test-3", "Hello");
        session.add_message("user", "Hello Hydra");
        session.add_message("hydra", "Hi! How can I help?");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "hydra");
    }

    #[test]
    fn test_session_complete() {
        let mut session = Session::new("test-4", "Task");
        assert_eq!(session.status, SessionStatus::Active);
        session.complete();
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_session_store_new_session() {
        let mut store = SessionStore::new();
        let id = store.new_session("Test task");
        assert_eq!(store.count(), 1);
        assert_eq!(store.active_session_id, Some(id.clone()));
        let session = store.active_session().unwrap();
        assert_eq!(session.title, "Test task");
    }

    #[test]
    fn test_session_store_add_message() {
        let mut store = SessionStore::new();
        store.new_session("Test");
        store.add_message("user", "Hello");
        store.add_message("hydra", "Hi");
        let session = store.active_session().unwrap();
        assert_eq!(session.messages.len(), 2);
    }

    #[test]
    fn test_session_store_switch() {
        let mut store = SessionStore::new();
        let id1 = store.new_session("First");
        let id2 = store.new_session("Second");
        assert_eq!(store.active_session_id, Some(id2.clone()));
        store.switch_to(&id1);
        assert_eq!(store.active_session_id, Some(id1));
    }

    #[test]
    fn test_session_store_sorted() {
        let mut store = SessionStore::new();
        store.new_session("First");
        store.new_session("Second");
        let sorted = store.sorted_sessions();
        assert_eq!(sorted.len(), 2);
        // Most recent first
        assert_eq!(sorted[0].title, "Second");
    }

    #[test]
    fn test_session_store_delete() {
        let mut store = SessionStore::new();
        let id = store.new_session("Delete me");
        assert_eq!(store.count(), 1);
        store.delete_session(&id);
        assert_eq!(store.count(), 0);
        assert_eq!(store.active_session_id, None);
    }

    #[test]
    fn test_session_store_filter_by_status() {
        let mut store = SessionStore::new();
        store.new_session("Active one");
        let id2 = store.new_session("Completed one");
        if let Some(s) = store.sessions.get_mut(&id2) {
            s.complete();
        }
        let active = store.sessions_by_status(SessionStatus::Active);
        let completed = store.sessions_by_status(SessionStatus::Completed);
        assert_eq!(active.len(), 1);
        assert_eq!(completed.len(), 1);
    }
}
