//! Chat interface component data.

use serde::{Deserialize, Serialize};

use crate::state::hydra::{ChatMessage, MessageRole};

/// Props for the chat component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatProps {
    pub messages: Vec<ChatMessage>,
    pub is_processing: bool,
    pub placeholder: String,
}

impl Default for ChatProps {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            is_processing: false,
            placeholder: "Ask Hydra anything...".into(),
        }
    }
}

/// Render data for a chat message bubble
#[derive(Debug, Clone)]
pub struct MessageBubble {
    pub content: String,
    pub is_user: bool,
    pub timestamp: String,
    pub tokens_label: Option<String>,
    pub css_class: &'static str,
}

impl MessageBubble {
    pub fn from_message(msg: &ChatMessage) -> Self {
        let is_user = msg.role == MessageRole::User;
        Self {
            content: msg.content.clone(),
            is_user,
            timestamp: msg.timestamp.clone(),
            tokens_label: msg.tokens_used.map(|t| format!("{} tokens", t)),
            css_class: if is_user {
                "message-user"
            } else {
                "message-hydra"
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_bubble_user() {
        let msg = ChatMessage {
            id: "1".into(),
            role: MessageRole::User,
            content: "Hello".into(),
            timestamp: "2026-03-07T00:00:00Z".into(),
            run_id: None,
            tokens_used: None,
        };
        let bubble = MessageBubble::from_message(&msg);
        assert!(bubble.is_user);
        assert_eq!(bubble.css_class, "message-user");
        assert!(bubble.tokens_label.is_none());
    }

    #[test]
    fn test_message_bubble_hydra() {
        let msg = ChatMessage {
            id: "2".into(),
            role: MessageRole::Hydra,
            content: "Hi!".into(),
            timestamp: "2026-03-07T00:00:00Z".into(),
            run_id: Some("run-1".into()),
            tokens_used: Some(150),
        };
        let bubble = MessageBubble::from_message(&msg);
        assert!(!bubble.is_user);
        assert_eq!(bubble.css_class, "message-hydra");
        assert_eq!(bubble.tokens_label.as_deref(), Some("150 tokens"));
    }
}
