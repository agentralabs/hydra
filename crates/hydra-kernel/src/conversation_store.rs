//! Conversation persistence — saves exchanges to disk for /resume.
//! Stored as JSON lines in ~/.hydra/data/conversations/<session_id>.jsonl.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One exchange in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exchange {
    pub input: String,
    pub response: String,
    pub tokens: usize,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Manages conversation persistence.
pub struct ConversationStore {
    session_id: String,
    exchanges: Vec<Exchange>,
    file_path: PathBuf,
}

impl ConversationStore {
    pub fn new(session_id: &str) -> Self {
        let dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".hydra/data/conversations");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("hydra: conversation dir create failed: {e}");
        }
        let file_path = dir.join(format!("{session_id}.jsonl"));

        Self {
            session_id: session_id.into(),
            exchanges: Vec::new(),
            file_path,
        }
    }

    /// Record an exchange and persist to disk.
    pub fn record(&mut self, input: &str, response: &str, tokens: usize, duration_ms: u64) {
        let exchange = Exchange {
            input: input.into(),
            response: response.into(),
            tokens,
            duration_ms,
            timestamp: Utc::now(),
        };

        // Append to file (JSON lines)
        if let Ok(json) = serde_json::to_string(&exchange) {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
            {
                if let Err(e) = writeln!(file, "{json}") {
                    eprintln!("hydra: conversation write failed: {e}");
                }
                if let Err(e) = file.flush() {
                    eprintln!("hydra: conversation flush failed: {e}");
                }
            }
        }

        self.exchanges.push(exchange);
    }

    /// Load the most recent conversation session.
    pub fn load_latest() -> Option<Vec<Exchange>> {
        let dir = dirs::home_dir()?.join(".hydra/data/conversations");
        if !dir.exists() {
            return None;
        }

        let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if latest.as_ref().is_none_or(|(t, _)| modified > *t) {
                            latest = Some((modified, entry.path()));
                        }
                    }
                }
            }
        }

        let path = latest?.1;
        let content = std::fs::read_to_string(&path).ok()?;
        let exchanges: Vec<Exchange> = content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();

        if exchanges.is_empty() {
            None
        } else {
            Some(exchanges)
        }
    }

    /// List available conversation sessions.
    pub fn list_sessions() -> Vec<(String, usize, DateTime<Utc>)> {
        let dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".hydra/data/conversations");
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".jsonl") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let count = content.lines().count();
                    let last_ts = content
                        .lines()
                        .last()
                        .and_then(|l| serde_json::from_str::<Exchange>(l).ok())
                        .map(|e| e.timestamp)
                        .unwrap_or_else(Utc::now);
                    sessions.push((name.trim_end_matches(".jsonl").into(), count, last_ts));
                }
            }
        }
        sessions.sort_by(|a, b| b.2.cmp(&a.2));
        sessions
    }

    pub fn exchange_count(&self) -> usize {
        self.exchanges.len()
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_count() {
        let id = format!("test-{}", uuid::Uuid::new_v4());
        let mut store = ConversationStore::new(&id);
        store.record("hello", "hi there", 10, 50);
        store.record("how are you", "I'm great", 15, 30);
        assert_eq!(store.exchange_count(), 2);

        // Cleanup
        let _ = std::fs::remove_file(&store.file_path);
    }

    #[test]
    fn list_sessions_doesnt_panic() {
        let sessions = ConversationStore::list_sessions();
        // Just verify it runs without error
        assert!(sessions.len() < 10_000);
    }
}
