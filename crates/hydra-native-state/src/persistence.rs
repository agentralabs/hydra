//! Chat persistence — wires hydra-db MessageStore into the desktop app.

use std::path::PathBuf;

use hydra_db::{Conversation, HydraDb, Message, MessageRole, MessageStore};

use chrono::Utc;

/// Persistent chat storage backed by SQLite
pub struct ChatPersistence {
    db: HydraDb,
    messages: MessageStore,
    current_conversation_id: parking_lot::Mutex<Option<String>>,
}

impl ChatPersistence {
    /// Initialize with database at ~/.hydra/hydra.db
    pub fn init() -> Result<Self, String> {
        let db_path = Self::db_path();
        let db = HydraDb::init(&db_path).map_err(|e| format!("DB init failed: {}", e))?;
        let messages =
            MessageStore::new(db.connection()).map_err(|e| format!("MessageStore init failed: {}", e))?;
        Ok(Self {
            db,
            messages,
            current_conversation_id: parking_lot::Mutex::new(None),
        })
    }

    fn db_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".hydra").join("hydra.db")
    }

    /// Start a new conversation or resume the most recent one
    pub fn ensure_conversation(&self) -> String {
        let mut current = self.current_conversation_id.lock();
        if let Some(ref id) = *current {
            return id.clone();
        }

        // Try to resume most recent conversation
        if let Ok(convs) = self.messages.list_conversations() {
            if let Some(conv) = convs.first() {
                *current = Some(conv.id.clone());
                return conv.id.clone();
            }
        }

        // Create new conversation
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conv = Conversation {
            id: id.clone(),
            title: Some("New conversation".to_string()),
            created_at: now.clone(),
            updated_at: now,
        };
        let _ = self.messages.create_conversation(&conv);
        *current = Some(id.clone());
        id
    }

    /// Start a fresh conversation
    pub fn new_conversation(&self) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let conv = Conversation {
            id: id.clone(),
            title: Some("New conversation".to_string()),
            created_at: now.clone(),
            updated_at: now,
        };
        let _ = self.messages.create_conversation(&conv);
        let mut current = self.current_conversation_id.lock();
        *current = Some(id.clone());
        id
    }

    /// Save a message to the current conversation
    pub fn save_message(&self, role: &str, content: &str) {
        let conv_id = self.ensure_conversation();
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conv_id,
            role: if role == "user" {
                MessageRole::User
            } else {
                MessageRole::Hydra
            },
            content: content.to_string(),
            created_at: Utc::now().to_rfc3339(),
            run_id: None,
            metadata: None,
        };
        let _ = self.messages.add_message(&msg);
    }

    /// Load all messages for the current conversation
    /// Returns Vec<(role, content, css_class)> matching the UI signal format
    pub fn load_messages(&self) -> Vec<(String, String, String)> {
        let conv_id = self.ensure_conversation();
        match self.messages.get_conversation(&conv_id, None) {
            Ok(msgs) => msgs
                .iter()
                .map(|m| {
                    let role = m.role.as_str().to_string();
                    let css = if m.role == MessageRole::User {
                        "message user".to_string()
                    } else {
                        "message hydra".to_string()
                    };
                    (role, m.content.clone(), css)
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// List all conversations for sidebar
    pub fn list_conversations(&self) -> Vec<(String, String)> {
        match self.messages.list_conversations() {
            Ok(convs) => convs
                .iter()
                .map(|c| {
                    (
                        c.id.clone(),
                        c.title
                            .clone()
                            .unwrap_or_else(|| "Untitled".to_string()),
                    )
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Switch to a different conversation
    pub fn switch_conversation(&self, id: &str) -> Vec<(String, String, String)> {
        {
            let mut current = self.current_conversation_id.lock();
            *current = Some(id.to_string());
        }
        self.load_messages()
    }

    /// Delete a conversation
    pub fn delete_conversation(&self, id: &str) {
        let _ = self.messages.delete_conversation(id);
        let mut current = self.current_conversation_id.lock();
        if current.as_deref() == Some(id) {
            *current = None;
        }
    }

    /// Update conversation title based on first message
    pub fn update_title(&self, title: &str) {
        // MessageStore does not expose update_title yet — track for future enhancement
        let _ = title;
    }
}

impl Clone for ChatPersistence {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            messages: self.messages.clone(),
            current_conversation_id: parking_lot::Mutex::new(
                self.current_conversation_id.lock().clone(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ChatPersistence with in-memory DB for testing (no filesystem)
    fn test_persistence() -> ChatPersistence {
        let db = HydraDb::in_memory().unwrap();
        let messages = MessageStore::new(db.connection()).unwrap();
        ChatPersistence {
            db,
            messages,
            current_conversation_id: parking_lot::Mutex::new(None),
        }
    }

    #[test]
    fn ensure_conversation_creates_one() {
        let p = test_persistence();
        let id = p.ensure_conversation();
        assert!(!id.is_empty());
        // Calling again returns the same ID
        assert_eq!(p.ensure_conversation(), id);
    }

    #[test]
    fn save_and_load_messages() {
        let p = test_persistence();
        p.save_message("user", "hello");
        p.save_message("hydra", "hi there");
        let msgs = p.load_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].0, "user");
        assert_eq!(msgs[0].1, "hello");
        assert_eq!(msgs[0].2, "message user");
        assert_eq!(msgs[1].0, "hydra");
        assert_eq!(msgs[1].1, "hi there");
        assert_eq!(msgs[1].2, "message hydra");
    }

    #[test]
    fn new_conversation_resets() {
        let p = test_persistence();
        p.save_message("user", "first conv msg");
        let id1 = p.ensure_conversation();
        let id2 = p.new_conversation();
        assert_ne!(id1, id2);
        // New conversation has no messages
        let msgs = p.load_messages();
        assert!(msgs.is_empty());
    }

    #[test]
    fn switch_conversation_loads_correct_messages() {
        let p = test_persistence();
        p.save_message("user", "conv1 msg");
        let id1 = p.ensure_conversation();
        let id2 = p.new_conversation();
        p.save_message("user", "conv2 msg");

        let msgs = p.switch_conversation(&id1);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].1, "conv1 msg");

        let msgs = p.switch_conversation(&id2);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].1, "conv2 msg");
    }

    #[test]
    fn list_conversations_returns_all() {
        let p = test_persistence();
        p.save_message("user", "a");
        p.new_conversation();
        p.save_message("user", "b");
        let convs = p.list_conversations();
        assert_eq!(convs.len(), 2);
    }

    #[test]
    fn delete_conversation_removes_it() {
        let p = test_persistence();
        p.save_message("user", "doomed");
        let id = p.ensure_conversation();
        p.delete_conversation(&id);
        let convs = p.list_conversations();
        assert!(convs.is_empty());
    }
}
