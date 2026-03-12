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
#[path = "messages_tests.rs"]
mod tests;
