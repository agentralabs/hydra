use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Voice pipeline state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceState {
    /// No voice activity — waiting for wake word
    Idle,
    /// Wake word detected — transitioning to listening
    WakeDetected,
    /// Actively listening to user speech
    Listening,
    /// Processing speech → text → intent
    Processing,
    /// Speaking response via TTS
    Speaking,
    /// Voice disabled
    Disabled,
}

/// Voice session tracking
pub struct VoiceSession {
    state: AtomicU8,
    session_start: parking_lot::Mutex<Option<Instant>>,
}

impl VoiceSession {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(Self::state_to_u8(VoiceState::Idle)),
            session_start: parking_lot::Mutex::new(None),
        }
    }

    pub fn state(&self) -> VoiceState {
        Self::u8_to_state(self.state.load(Ordering::SeqCst))
    }

    /// Transition to a new state with validation
    pub fn transition(&self, target: VoiceState) -> bool {
        let current = self.state();
        let valid = match (current, target) {
            (VoiceState::Idle, VoiceState::WakeDetected) => true,
            (VoiceState::WakeDetected, VoiceState::Listening) => true,
            (VoiceState::Listening, VoiceState::Processing) => true,
            (VoiceState::Processing, VoiceState::Speaking) => true,
            (VoiceState::Speaking, VoiceState::Idle) => true,
            // Barge-in: user speaks during TTS
            (VoiceState::Speaking, VoiceState::Listening) => true,
            // Timeout/cancel paths
            (VoiceState::Listening, VoiceState::Idle) => true,
            (VoiceState::Processing, VoiceState::Idle) => true,
            (VoiceState::WakeDetected, VoiceState::Idle) => true,
            // Any → Disabled
            (_, VoiceState::Disabled) => true,
            // Disabled → Idle (re-enable)
            (VoiceState::Disabled, VoiceState::Idle) => true,
            _ => false,
        };
        if valid {
            self.state
                .store(Self::state_to_u8(target), Ordering::SeqCst);
            if target == VoiceState::WakeDetected {
                *self.session_start.lock() = Some(Instant::now());
            } else if target == VoiceState::Idle {
                *self.session_start.lock() = None;
            }
        }
        valid
    }

    /// Is voice collision happening? (speaking while should be listening)
    pub fn is_collision(&self) -> bool {
        self.state() == VoiceState::Speaking
    }

    /// Session duration if active
    pub fn session_duration(&self) -> Option<std::time::Duration> {
        self.session_start.lock().map(|s| s.elapsed())
    }

    fn state_to_u8(state: VoiceState) -> u8 {
        match state {
            VoiceState::Idle => 0,
            VoiceState::WakeDetected => 1,
            VoiceState::Listening => 2,
            VoiceState::Processing => 3,
            VoiceState::Speaking => 4,
            VoiceState::Disabled => 5,
        }
    }

    fn u8_to_state(val: u8) -> VoiceState {
        match val {
            0 => VoiceState::Idle,
            1 => VoiceState::WakeDetected,
            2 => VoiceState::Listening,
            3 => VoiceState::Processing,
            4 => VoiceState::Speaking,
            _ => VoiceState::Disabled,
        }
    }
}

impl Default for VoiceSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_state_idle_to_listening() {
        let session = VoiceSession::new();
        assert_eq!(session.state(), VoiceState::Idle, "Session starts Idle");

        // Idle → WakeDetected
        assert!(
            session.transition(VoiceState::WakeDetected),
            "Idle → WakeDetected must succeed"
        );
        assert_eq!(session.state(), VoiceState::WakeDetected);

        // WakeDetected → Listening
        assert!(
            session.transition(VoiceState::Listening),
            "WakeDetected → Listening must succeed"
        );
        assert_eq!(session.state(), VoiceState::Listening);
    }

    #[test]
    fn test_voice_state_full_cycle() {
        let session = VoiceSession::new();

        // Full forward path: Idle → WakeDetected → Listening → Processing → Speaking → Idle
        assert!(session.transition(VoiceState::WakeDetected));
        assert!(session.transition(VoiceState::Listening));
        assert!(session.transition(VoiceState::Processing));
        assert!(session.transition(VoiceState::Speaking));
        assert!(session.transition(VoiceState::Idle));

        assert_eq!(
            session.state(),
            VoiceState::Idle,
            "Must return to Idle after full cycle"
        );
        assert!(
            session.session_duration().is_none(),
            "Session duration must be cleared on return to Idle"
        );
    }

    #[test]
    fn test_voice_barge_in() {
        let session = VoiceSession::new();

        // Advance to Speaking
        session.transition(VoiceState::WakeDetected);
        session.transition(VoiceState::Listening);
        session.transition(VoiceState::Processing);
        session.transition(VoiceState::Speaking);

        assert_eq!(session.state(), VoiceState::Speaking);
        assert!(
            session.is_collision(),
            "Speaking state must report as collision"
        );

        // Barge-in: Speaking → Listening (user speaks over TTS)
        assert!(
            session.transition(VoiceState::Listening),
            "Barge-in transition Speaking → Listening must succeed"
        );
        assert_eq!(session.state(), VoiceState::Listening);
    }

    #[test]
    fn test_voice_silence_timeout() {
        let session = VoiceSession::new();

        // Advance to Listening
        session.transition(VoiceState::WakeDetected);
        session.transition(VoiceState::Listening);
        assert_eq!(session.state(), VoiceState::Listening);

        // Silence timeout: Listening → Idle (no speech detected)
        assert!(
            session.transition(VoiceState::Idle),
            "Silence timeout Listening → Idle must succeed"
        );
        assert_eq!(session.state(), VoiceState::Idle);
    }

    #[test]
    fn test_voice_invalid_transitions_rejected() {
        let session = VoiceSession::new();

        // Cannot jump directly Idle → Processing
        assert!(
            !session.transition(VoiceState::Processing),
            "Idle → Processing must be rejected"
        );
        assert_eq!(
            session.state(),
            VoiceState::Idle,
            "State must not change on invalid transition"
        );

        // Cannot jump Idle → Speaking
        assert!(!session.transition(VoiceState::Speaking));
        assert_eq!(session.state(), VoiceState::Idle);
    }

    #[test]
    fn test_voice_session_duration_tracked() {
        let session = VoiceSession::new();
        assert!(
            session.session_duration().is_none(),
            "No duration before wake word"
        );

        session.transition(VoiceState::WakeDetected);
        // Duration begins at WakeDetected
        let dur = session.session_duration();
        assert!(dur.is_some(), "Duration must be tracked after WakeDetected");

        session.transition(VoiceState::Listening);
        session.transition(VoiceState::Processing);
        session.transition(VoiceState::Speaking);
        session.transition(VoiceState::Idle);
        assert!(
            session.session_duration().is_none(),
            "Duration must clear on Idle"
        );
    }

    #[test]
    fn test_voice_disable_from_any_state() {
        // Any state → Disabled must always succeed
        for start in [
            VoiceState::Idle,
            VoiceState::WakeDetected,
            VoiceState::Listening,
            VoiceState::Processing,
            VoiceState::Speaking,
        ] {
            let session = VoiceSession::new();
            // Get to starting state
            if start != VoiceState::Idle {
                session.transition(VoiceState::WakeDetected);
            }
            if matches!(
                start,
                VoiceState::Listening | VoiceState::Processing | VoiceState::Speaking
            ) {
                session.transition(VoiceState::Listening);
            }
            if matches!(start, VoiceState::Processing | VoiceState::Speaking) {
                session.transition(VoiceState::Processing);
            }
            if start == VoiceState::Speaking {
                session.transition(VoiceState::Speaking);
            }

            assert!(
                session.transition(VoiceState::Disabled),
                "Any state → Disabled must succeed (got {:?})",
                session.state()
            );
        }
    }
}
