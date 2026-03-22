//! SessionContinuity — seamless handoff between devices.
//! A conversation that starts on Meta glasses continues on the
//! desktop TUI with no gap, no context loss, no re-introduction.

use crate::{
    constants::SESSION_HANDOFF_TIMEOUT_SECONDS,
    errors::ReachError,
    session::DeviceSession,
    surface::OutputMode,
};
use serde::{Deserialize, Serialize};

/// A handoff package — the context needed to continue a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPackage {
    /// The device the session is coming from.
    pub source_device_id: String,
    /// The device the session is going to.
    pub target_device_id: String,
    /// The session being handed off.
    pub session_id: String,
    /// The last N message IDs for context reconstruction.
    pub context_tail: Vec<String>,
    /// The active task description (if any).
    pub active_task: Option<String>,
    /// The current belief count (informational).
    pub belief_count: u64,
    /// When the handoff package was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl HandoffPackage {
    /// Check whether this handoff package is still valid (not expired).
    pub fn is_valid(&self) -> bool {
        let age = (chrono::Utc::now() - self.created_at).num_seconds();
        age < SESSION_HANDOFF_TIMEOUT_SECONDS as i64
    }
}

/// Prepare a handoff package from an active session.
pub fn prepare_handoff(
    session: &DeviceSession,
    target_device: &str,
    active_task: Option<String>,
    belief_count: u64,
) -> HandoffPackage {
    HandoffPackage {
        source_device_id: session.device_id.clone(),
        target_device_id: target_device.to_string(),
        session_id: session.id.clone(),
        context_tail: session.context_tail.clone(),
        active_task,
        belief_count,
        created_at: chrono::Utc::now(),
    }
}

/// Apply a handoff package to create a new session on the target device.
pub fn apply_handoff(
    package: &HandoffPackage,
    output_mode: OutputMode,
) -> Result<DeviceSession, ReachError> {
    if !package.is_valid() {
        return Err(ReachError::HandoffFailed {
            reason: "handoff package expired".to_string(),
        });
    }

    let mut session = DeviceSession::new(package.target_device_id.clone(), output_mode);
    // Restore context tail from handoff
    session.context_tail = package.context_tail.clone();

    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_prepared_and_applied() {
        let source_session = DeviceSession::new("glasses-001", OutputMode::VoiceOnly);
        let package = prepare_handoff(
            &source_session,
            "desktop-001",
            Some("building Phase 14".into()),
            565,
        );

        assert_eq!(package.source_device_id, "glasses-001");
        assert_eq!(package.target_device_id, "desktop-001");
        assert!(package.is_valid());

        let new_session = apply_handoff(&package, OutputMode::FullCockpit).unwrap();
        assert_eq!(new_session.device_id, "desktop-001");
    }

    #[test]
    fn handoff_carries_context_tail() {
        let mut session = DeviceSession::new("glasses", OutputMode::VoiceOnly);
        session.record_message("msg-001");
        session.record_message("msg-002");

        let package = prepare_handoff(&session, "desktop", None, 0);
        let new_session = apply_handoff(&package, OutputMode::FullCockpit).unwrap();

        assert_eq!(new_session.context_tail.len(), 2);
    }

    #[test]
    fn handoff_has_session_id() {
        let session = DeviceSession::new("glasses", OutputMode::VoiceOnly);
        let package = prepare_handoff(&session, "desktop", None, 0);
        assert!(!package.session_id.is_empty());
    }
}
