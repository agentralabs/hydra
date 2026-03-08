use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::store::DbError;

// ═══════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Hydra,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Hydra => "hydra",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "hydra" => Some(Self::Hydra),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at: String,
    pub run_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ═══════════════════════════════════════════════════════════
// SCHEMA
// ═══════════════════════════════════════════════════════════

pub const CREATE_MESSAGE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    title TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user','hydra')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    run_id TEXT,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
CREATE INDEX IF NOT EXISTS idx_messages_run ON messages(run_id) WHERE run_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at);
"#;

// ═══════════════════════════════════════════════════════════
// MESSAGE STORE
// ═══════════════════════════════════════════════════════════

pub struct MessageStore {
    conn: Arc<Mutex<Connection>>,
}

impl MessageStore {
    /// Create a MessageStore sharing the same connection as HydraDb
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, DbError> {
        {
            let c = conn.lock();
            c.execute_batch(CREATE_MESSAGE_TABLES)?;
        }
        Ok(Self { conn })
    }

    /// Create an in-memory MessageStore for testing
    pub fn in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(CREATE_MESSAGE_TABLES)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ═══════════════════════════════════════════════════════
    // CONVERSATIONS
    // ═══════════════════════════════════════════════════════

    pub fn create_conversation(&self, conv: &Conversation) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![conv.id, conv.title, conv.created_at, conv.updated_at],
        )?;
        Ok(())
    }

    pub fn get_conversation_info(&self, id: &str) -> Result<Conversation, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, title, created_at, updated_at FROM conversations WHERE id = ?1",
            params![id],
            |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DbError::NotFound(format!("Conversation {id}"))
            }
            other => DbError::Sqlite(other),
        })
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )?;
        let iter = stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        // Messages are deleted via ON DELETE CASCADE
        let affected = conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Conversation {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // MESSAGES
    // ═══════════════════════════════════════════════════════

    pub fn add_message(&self, msg: &Message) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let metadata_str = msg.metadata.as_ref().map(|v| v.to_string());
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content, created_at, run_id, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![msg.id, msg.conversation_id, msg.role.as_str(), msg.content, msg.created_at, msg.run_id, metadata_str],
        )?;
        // Update conversation's updated_at
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, msg.conversation_id],
        )?;
        Ok(())
    }

    pub fn get_conversation(
        &self,
        conversation_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Message>, DbError> {
        let conn = self.conn.lock();
        let limit_val = limit.unwrap_or(1000) as i64;
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, run_id, metadata FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC LIMIT ?2",
        )?;
        let iter = stmt.query_map(params![conversation_id, limit_val], |row| {
            let role_str: String = row.get(2)?;
            let metadata_str: Option<String> = row.get(6)?;
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: MessageRole::from_str(&role_str).unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at: row.get(4)?,
                run_id: row.get(5)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }

    pub fn get_message(&self, conversation_id: &str, message_id: &str) -> Result<Message, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, conversation_id, role, content, created_at, run_id, metadata FROM messages WHERE id = ?1 AND conversation_id = ?2",
            params![message_id, conversation_id],
            |row| {
                let role_str: String = row.get(2)?;
                let metadata_str: Option<String> = row.get(6)?;
                Ok(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: MessageRole::from_str(&role_str).unwrap_or(MessageRole::User),
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                    run_id: row.get(5)?,
                    metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DbError::NotFound(format!("Message {message_id} in conversation {conversation_id}"))
            }
            other => DbError::Sqlite(other),
        })
    }

    pub fn get_recent(&self, limit: usize) -> Result<Vec<Message>, DbError> {
        let conn = self.conn.lock();
        let limit_val = limit as i64;
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, run_id, metadata FROM messages ORDER BY created_at DESC LIMIT ?1",
        )?;
        let iter = stmt.query_map(params![limit_val], |row| {
            let role_str: String = row.get(2)?;
            let metadata_str: Option<String> = row.get(6)?;
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: MessageRole::from_str(&role_str).unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at: row.get(4)?,
                run_id: row.get(5)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }

    pub fn search(&self, query: &str) -> Result<Vec<Message>, DbError> {
        let conn = self.conn.lock();
        let pattern = format!("%{query}%");
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, created_at, run_id, metadata FROM messages WHERE content LIKE ?1 ORDER BY created_at DESC",
        )?;
        let iter = stmt.query_map(params![pattern], |row| {
            let role_str: String = row.get(2)?;
            let metadata_str: Option<String> = row.get(6)?;
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                role: MessageRole::from_str(&role_str).unwrap_or(MessageRole::User),
                content: row.get(3)?,
                created_at: row.get(4)?,
                run_id: row.get(5)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }
}

impl Clone for MessageStore {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_conversation(id: &str) -> Conversation {
        let now = Utc::now().to_rfc3339();
        Conversation {
            id: id.into(),
            title: Some(format!("Conv {}", id)),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    fn make_message(id: &str, conv_id: &str, role: MessageRole) -> Message {
        Message {
            id: id.into(),
            conversation_id: conv_id.into(),
            role,
            content: format!("Message {}", id),
            created_at: Utc::now().to_rfc3339(),
            run_id: None,
            metadata: None,
        }
    }

    // --- MessageRole ---

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Hydra.as_str(), "hydra");
    }

    #[test]
    fn test_message_role_from_str() {
        assert_eq!(MessageRole::from_str("user"), Some(MessageRole::User));
        assert_eq!(MessageRole::from_str("hydra"), Some(MessageRole::Hydra));
        assert_eq!(MessageRole::from_str("invalid"), None);
    }

    #[test]
    fn test_message_role_serde() {
        for role in [MessageRole::User, MessageRole::Hydra] {
            let json = serde_json::to_string(&role).unwrap();
            let restored: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, role);
        }
    }

    // --- MessageStore Init ---

    #[test]
    fn test_in_memory() {
        let store = MessageStore::in_memory().unwrap();
        let convs = store.list_conversations().unwrap();
        assert!(convs.is_empty());
    }

    #[test]
    fn test_clone_shares_connection() {
        let store = MessageStore::in_memory().unwrap();
        let store2 = store.clone();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let convs = store2.list_conversations().unwrap();
        assert_eq!(convs.len(), 1);
    }

    // --- Conversations ---

    #[test]
    fn test_create_and_get_conversation() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let conv = store.get_conversation_info("c1").unwrap();
        assert_eq!(conv.title, Some("Conv c1".into()));
    }

    #[test]
    fn test_get_conversation_not_found() {
        let store = MessageStore::in_memory().unwrap();
        let err = store.get_conversation_info("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_conversations() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.create_conversation(&make_conversation("c2")).unwrap();
        let convs = store.list_conversations().unwrap();
        assert_eq!(convs.len(), 2);
    }

    #[test]
    fn test_delete_conversation() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.delete_conversation("c1").unwrap();
        assert!(matches!(store.get_conversation_info("c1").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_delete_conversation_not_found() {
        let store = MessageStore::in_memory().unwrap();
        let err = store.delete_conversation("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- Messages ---

    #[test]
    fn test_add_and_get_message() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let msg = make_message("m1", "c1", MessageRole::User);
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert_eq!(fetched.content, "Message m1");
        assert_eq!(fetched.role, MessageRole::User);
    }

    #[test]
    fn test_get_message_not_found() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let err = store.get_message("c1", "nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_get_conversation_messages() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        let msgs = store.get_conversation("c1", None).unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn test_get_conversation_messages_with_limit() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        store.add_message(&make_message("m3", "c1", MessageRole::User)).unwrap();
        let msgs = store.get_conversation("c1", Some(2)).unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn test_get_recent() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.add_message(&make_message("m2", "c1", MessageRole::Hydra)).unwrap();
        let recent = store.get_recent(1).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn test_search() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.content = "hello world".into();
        store.add_message(&msg).unwrap();
        let mut msg2 = make_message("m2", "c1", MessageRole::Hydra);
        msg2.content = "goodbye".into();
        store.add_message(&msg2).unwrap();
        let results = store.search("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[test]
    fn test_search_no_results() {
        let store = MessageStore::in_memory().unwrap();
        let results = store.search("nonexistent").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_message_with_metadata() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.metadata = Some(serde_json::json!({"key": "value"}));
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert!(fetched.metadata.is_some());
        assert_eq!(fetched.metadata.unwrap()["key"], "value");
    }

    #[test]
    fn test_message_with_run_id() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        let mut msg = make_message("m1", "c1", MessageRole::User);
        msg.run_id = Some("run-123".into());
        store.add_message(&msg).unwrap();
        let fetched = store.get_message("c1", "m1").unwrap();
        assert_eq!(fetched.run_id, Some("run-123".into()));
    }

    #[test]
    fn test_delete_conversation_cascades_messages() {
        let store = MessageStore::in_memory().unwrap();
        store.create_conversation(&make_conversation("c1")).unwrap();
        store.add_message(&make_message("m1", "c1", MessageRole::User)).unwrap();
        store.delete_conversation("c1").unwrap();
        let msgs = store.get_conversation("c1", None).unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_conversation_no_title() {
        let store = MessageStore::in_memory().unwrap();
        let mut conv = make_conversation("c1");
        conv.title = None;
        store.create_conversation(&conv).unwrap();
        let fetched = store.get_conversation_info("c1").unwrap();
        assert!(fetched.title.is_none());
    }

    // --- Serde ---

    #[test]
    fn test_message_serde() {
        let msg = make_message("m1", "c1", MessageRole::User);
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "m1");
    }

    #[test]
    fn test_conversation_serde() {
        let conv = make_conversation("c1");
        let json = serde_json::to_string(&conv).unwrap();
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "c1");
    }
}
