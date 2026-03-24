//! Session manager — auto-save exchanges, /resume, session listing.
//! Wraps hydra_kernel::conversation_store for TUI integration.

use crate::stream_types::StreamItem;
use crate::v2::modal::SessionEntry;

/// TUI session manager.
pub struct SessionManager {
    store: hydra_kernel::conversation_store::ConversationStore,
    auto_save: bool,
}

impl SessionManager {
    pub fn new(auto_save: bool) -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        Self {
            store: hydra_kernel::conversation_store::ConversationStore::new(&session_id),
            auto_save,
        }
    }

    /// Record an exchange (auto-called after each LLM response).
    pub fn record(&mut self, input: &str, response: &str, tokens: usize, duration_ms: u64) {
        if self.auto_save {
            self.store.record(input, response, tokens, duration_ms);
        }
    }

    /// Load the latest conversation as stream items.
    pub fn resume_latest(&self) -> Vec<StreamItem> {
        match hydra_kernel::conversation_store::ConversationStore::load_latest() {
            Some(exchanges) => {
                let mut items = Vec::new();
                items.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Resumed conversation ({} exchanges)", exchanges.len()),
                    timestamp: chrono::Utc::now(),
                });

                for ex in exchanges.iter().rev().take(10).rev() {
                    items.push(StreamItem::UserMessage {
                        id: uuid::Uuid::new_v4(),
                        text: ex.input.clone(),
                        timestamp: ex.timestamp,
                    });
                    items.push(StreamItem::AssistantText {
                        id: uuid::Uuid::new_v4(),
                        text: ex.response.clone(),
                        timestamp: ex.timestamp,
                    });
                }

                if exchanges.len() > 10 {
                    items.push(StreamItem::SystemNotification {
                        id: uuid::Uuid::new_v4(),
                        content: format!("({} earlier exchanges not shown)", exchanges.len() - 10),
                        timestamp: chrono::Utc::now(),
                    });
                }

                items
            }
            None => vec![StreamItem::SystemNotification {
                id: uuid::Uuid::new_v4(),
                content: "No saved conversations found.".into(),
                timestamp: chrono::Utc::now(),
            }],
        }
    }

    /// List sessions for the session list modal.
    pub fn list_sessions(&self) -> Vec<SessionEntry> {
        hydra_kernel::conversation_store::ConversationStore::list_sessions()
            .into_iter()
            .map(|(id, count, ts)| SessionEntry {
                id,
                date: ts.format("%Y-%m-%d %H:%M").to_string(),
                exchange_count: count,
                preview: format!("{count} exchanges"),
            })
            .collect()
    }

    pub fn exchange_count(&self) -> usize {
        self.store.exchange_count()
    }

    pub fn session_id(&self) -> &str {
        self.store.session_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_creates() {
        let mgr = SessionManager::new(false);
        assert_eq!(mgr.exchange_count(), 0);
    }

    #[test]
    fn resume_empty_returns_notification() {
        let mgr = SessionManager::new(false);
        let items = mgr.resume_latest();
        assert!(!items.is_empty());
    }

    #[test]
    fn list_sessions_doesnt_panic() {
        let mgr = SessionManager::new(false);
        let _ = mgr.list_sessions();
    }
}
