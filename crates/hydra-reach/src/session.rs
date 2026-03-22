//! DeviceSession — one device's active conversation context.
//! Isolated per device. Transferable via session continuity.

use crate::surface::OutputMode;
use serde::{Deserialize, Serialize};

/// The state of a device session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is actively being used.
    Active,
    /// Session is idle (no recent messages).
    Idle,
    /// Session is being transferred to another device.
    Transferring,
    /// Session has been disconnected.
    Disconnected,
}

/// One device's session — the conversational context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    /// Unique session identifier.
    pub id: String,
    /// The device this session belongs to.
    pub device_id: String,
    /// The output mode for this session.
    pub output_mode: OutputMode,
    /// The current session state.
    pub state: SessionState,
    /// Last N message IDs from the conversation (for continuity).
    pub context_tail: Vec<String>,
    /// When the session was started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last active.
    pub last_active: chrono::DateTime<chrono::Utc>,
    /// Total messages in this session.
    pub message_count: u64,
}

impl DeviceSession {
    /// Create a new active device session.
    pub fn new(device_id: impl Into<String>, output_mode: OutputMode) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: device_id.into(),
            output_mode,
            state: SessionState::Active,
            context_tail: Vec::new(),
            started_at: now,
            last_active: now,
            message_count: 0,
        }
    }

    /// Record a message in the context tail.
    ///
    /// Keeps last 20 messages for handoff context.
    pub fn record_message(&mut self, message_id: impl Into<String>) {
        self.message_count += 1;
        self.last_active = chrono::Utc::now();
        self.context_tail.push(message_id.into());
        if self.context_tail.len() > 20 {
            self.context_tail.remove(0);
        }
    }

    /// Check whether this session is active or idle.
    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active | SessionState::Idle)
    }

    /// Return the session duration in seconds.
    pub fn duration_seconds(&self) -> i64 {
        (chrono::Utc::now() - self.started_at).num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_is_active() {
        let s = DeviceSession::new("device-001", OutputMode::FullCockpit);
        assert!(s.is_active());
        assert_eq!(s.message_count, 0);
    }

    #[test]
    fn context_tail_capped_at_20() {
        let mut s = DeviceSession::new("d1", OutputMode::TextStream);
        for i in 0..25 {
            s.record_message(format!("msg-{}", i));
        }
        assert!(s.context_tail.len() <= 20);
        assert_eq!(s.message_count, 25);
    }

    #[test]
    fn session_has_uuid_id() {
        let s = DeviceSession::new("d1", OutputMode::FullCockpit);
        assert!(!s.id.is_empty());
    }

    #[test]
    fn duration_is_non_negative() {
        let s = DeviceSession::new("d1", OutputMode::FullCockpit);
        assert!(s.duration_seconds() >= 0);
    }
}
