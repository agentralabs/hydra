// hydra-collab: Multi-agent collaboration primitives

use std::collections::HashMap;

/// Collaboration session between agents
#[derive(Debug, Clone)]
pub struct CollabSession {
    pub id: String,
    pub agents: Vec<String>,
    pub status: CollabStatus,
    pub created_at: String,
}

/// Status of a collaboration session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabStatus {
    /// Session is being set up
    Initializing,
    /// Session is active
    Active,
    /// Session is paused
    Paused,
    /// Session is completed
    Completed,
    /// Session failed
    Failed,
}

impl CollabStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initializing => "initializing",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// A message in a collaboration channel
#[derive(Debug, Clone)]
pub struct CollabMessage {
    pub id: String,
    pub session_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub message_type: MessageType,
}

/// Type of collaboration message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Regular text message
    Text,
    /// Action proposal requiring consensus
    Proposal,
    /// Vote on a proposal
    Vote,
    /// System notification
    System,
    /// Result of an action
    Result,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Proposal => "proposal",
            Self::Vote => "vote",
            Self::System => "system",
            Self::Result => "result",
        }
    }
}

/// Collaboration manager
pub struct CollabManager {
    sessions: HashMap<String, CollabSession>,
    messages: HashMap<String, Vec<CollabMessage>>,
}

impl CollabManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    pub fn create_session(&mut self, id: &str, agents: Vec<String>) -> &CollabSession {
        let session = CollabSession {
            id: id.into(),
            agents,
            status: CollabStatus::Initializing,
            created_at: "now".into(),
        };
        self.sessions.insert(id.into(), session);
        self.messages.insert(id.into(), Vec::new());
        self.sessions.get(id).unwrap()
    }

    pub fn get_session(&self, id: &str) -> Option<&CollabSession> {
        self.sessions.get(id)
    }

    pub fn activate(&mut self, id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(id) {
            if session.status == CollabStatus::Initializing || session.status == CollabStatus::Paused
            {
                session.status = CollabStatus::Active;
                return true;
            }
        }
        false
    }

    pub fn pause(&mut self, id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(id) {
            if session.status == CollabStatus::Active {
                session.status = CollabStatus::Paused;
                return true;
            }
        }
        false
    }

    pub fn complete(&mut self, id: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(id) {
            if !session.status.is_terminal() {
                session.status = CollabStatus::Completed;
                return true;
            }
        }
        false
    }

    pub fn add_message(&mut self, msg: CollabMessage) {
        self.messages
            .entry(msg.session_id.clone())
            .or_default()
            .push(msg);
    }

    pub fn get_messages(&self, session_id: &str) -> &[CollabMessage] {
        self.messages
            .get(session_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn active_sessions(&self) -> Vec<&CollabSession> {
        self.sessions
            .values()
            .filter(|s| s.status.is_active())
            .collect()
    }
}

impl Default for CollabManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collab_status_as_str() {
        assert_eq!(CollabStatus::Initializing.as_str(), "initializing");
        assert_eq!(CollabStatus::Active.as_str(), "active");
        assert_eq!(CollabStatus::Paused.as_str(), "paused");
        assert_eq!(CollabStatus::Completed.as_str(), "completed");
        assert_eq!(CollabStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_collab_status_is_terminal() {
        assert!(!CollabStatus::Initializing.is_terminal());
        assert!(!CollabStatus::Active.is_terminal());
        assert!(!CollabStatus::Paused.is_terminal());
        assert!(CollabStatus::Completed.is_terminal());
        assert!(CollabStatus::Failed.is_terminal());
    }

    #[test]
    fn test_collab_status_is_active() {
        assert!(!CollabStatus::Initializing.is_active());
        assert!(CollabStatus::Active.is_active());
        assert!(!CollabStatus::Paused.is_active());
    }

    #[test]
    fn test_message_type_as_str() {
        assert_eq!(MessageType::Text.as_str(), "text");
        assert_eq!(MessageType::Proposal.as_str(), "proposal");
        assert_eq!(MessageType::Vote.as_str(), "vote");
        assert_eq!(MessageType::System.as_str(), "system");
        assert_eq!(MessageType::Result.as_str(), "result");
    }

    #[test]
    fn test_collab_manager_new() {
        let mgr = CollabManager::new();
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn test_collab_manager_default() {
        let mgr = CollabManager::default();
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn test_create_session() {
        let mut mgr = CollabManager::new();
        let session = mgr.create_session("s1", vec!["agent-a".into(), "agent-b".into()]);
        assert_eq!(session.id, "s1");
        assert_eq!(session.agents.len(), 2);
        assert_eq!(session.status, CollabStatus::Initializing);
    }

    #[test]
    fn test_get_session() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        assert!(mgr.get_session("s1").is_some());
        assert!(mgr.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_activate_session() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        assert!(mgr.activate("s1"));
        assert_eq!(mgr.get_session("s1").unwrap().status, CollabStatus::Active);
    }

    #[test]
    fn test_activate_already_active() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.activate("s1");
        assert!(!mgr.activate("s1")); // Already active
    }

    #[test]
    fn test_pause_session() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.activate("s1");
        assert!(mgr.pause("s1"));
        assert_eq!(mgr.get_session("s1").unwrap().status, CollabStatus::Paused);
    }

    #[test]
    fn test_pause_not_active() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        assert!(!mgr.pause("s1")); // Still initializing
    }

    #[test]
    fn test_resume_paused() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.activate("s1");
        mgr.pause("s1");
        assert!(mgr.activate("s1")); // Resume from paused
        assert_eq!(mgr.get_session("s1").unwrap().status, CollabStatus::Active);
    }

    #[test]
    fn test_complete_session() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.activate("s1");
        assert!(mgr.complete("s1"));
        assert_eq!(
            mgr.get_session("s1").unwrap().status,
            CollabStatus::Completed
        );
    }

    #[test]
    fn test_complete_already_terminal() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.complete("s1");
        assert!(!mgr.complete("s1")); // Already completed
    }

    #[test]
    fn test_add_and_get_messages() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["agent-a".into()]);
        mgr.add_message(CollabMessage {
            id: "m1".into(),
            session_id: "s1".into(),
            sender: "agent-a".into(),
            content: "Hello!".into(),
            timestamp: "now".into(),
            message_type: MessageType::Text,
        });
        let msgs = mgr.get_messages("s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Hello!");
    }

    #[test]
    fn test_get_messages_empty() {
        let mgr = CollabManager::new();
        assert!(mgr.get_messages("nonexistent").is_empty());
    }

    #[test]
    fn test_active_sessions() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["a".into()]);
        mgr.create_session("s2", vec!["b".into()]);
        mgr.activate("s1");
        let active = mgr.active_sessions();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "s1");
    }

    #[test]
    fn test_session_count() {
        let mut mgr = CollabManager::new();
        assert_eq!(mgr.session_count(), 0);
        mgr.create_session("s1", vec!["a".into()]);
        assert_eq!(mgr.session_count(), 1);
        mgr.create_session("s2", vec!["b".into()]);
        assert_eq!(mgr.session_count(), 2);
    }

    #[test]
    fn test_activate_nonexistent() {
        let mut mgr = CollabManager::new();
        assert!(!mgr.activate("nonexistent"));
    }

    #[test]
    fn test_pause_nonexistent() {
        let mut mgr = CollabManager::new();
        assert!(!mgr.pause("nonexistent"));
    }

    #[test]
    fn test_complete_nonexistent() {
        let mut mgr = CollabManager::new();
        assert!(!mgr.complete("nonexistent"));
    }

    #[test]
    fn test_multiple_messages() {
        let mut mgr = CollabManager::new();
        mgr.create_session("s1", vec!["a".into()]);
        for i in 0..5 {
            mgr.add_message(CollabMessage {
                id: format!("m{}", i),
                session_id: "s1".into(),
                sender: "a".into(),
                content: format!("msg {}", i),
                timestamp: "now".into(),
                message_type: MessageType::Text,
            });
        }
        assert_eq!(mgr.get_messages("s1").len(), 5);
    }
}
