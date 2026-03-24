//! O17 Voice Session — multi-turn conversational state tracking.
//! Manages Dormant→Listening→Processing→Speaking state machine.
//! Tracks turn count, interrupted messages (EC-17.3), and auto-timeout.

use std::time::Instant;
use crate::constants;

/// Voice presence state — the 4-state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePresenceState {
    /// Wake word detector active, <1% CPU. Waiting for "Hydra."
    Dormant,
    /// Full STT active, processing speech input.
    Listening,
    /// CognitiveLoop running on transcript.
    Processing,
    /// TTS output active, speaking response.
    Speaking,
}

impl VoicePresenceState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dormant => "dormant",
            Self::Listening => "listening",
            Self::Processing => "processing",
            Self::Speaking => "speaking",
        }
    }
}

/// Tracks a multi-turn voice conversation session.
pub struct VoiceSession {
    pub state: VoicePresenceState,
    pub started_at: Instant,
    pub last_activity: Instant,
    pub turn_count: u32,
    /// EC-17.3: Messages interrupted by barge-in, queued for replay.
    pub interrupted_messages: Vec<String>,
    /// Seconds of silence before returning to Dormant.
    pub active_timeout_secs: u64,
}

impl VoiceSession {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: VoicePresenceState::Dormant,
            started_at: now,
            last_activity: now,
            turn_count: 0,
            interrupted_messages: Vec::new(),
            active_timeout_secs: constants::SESSION_TIMEOUT_SECS,
        }
    }

    /// Wake word detected → Dormant → Listening.
    pub fn activate(&mut self) {
        self.state = VoicePresenceState::Listening;
        self.started_at = Instant::now();
        self.last_activity = Instant::now();
        self.turn_count = 0;
        eprintln!("hydra-voice: session activated → listening");
    }

    /// Silence detected after speech → Listening → Processing.
    pub fn speech_complete(&mut self) {
        self.state = VoicePresenceState::Processing;
        self.last_activity = Instant::now();
        self.turn_count += 1;
        eprintln!("hydra-voice: speech complete (turn {})", self.turn_count);
    }

    /// LLM response ready → Processing → Speaking.
    pub fn response_ready(&mut self) {
        self.state = VoicePresenceState::Speaking;
        self.last_activity = Instant::now();
    }

    /// TTS finished → Speaking → Listening (multi-turn: wait for follow-up).
    pub fn done_speaking(&mut self) {
        self.state = VoicePresenceState::Listening;
        self.last_activity = Instant::now();
    }

    /// Go back to dormant (timeout or explicit exit).
    pub fn go_dormant(&mut self) {
        self.state = VoicePresenceState::Dormant;
        eprintln!("hydra-voice: session ended after {} turns", self.turn_count);
    }

    /// Check if the multi-turn timeout has elapsed (call periodically).
    /// Returns true if state should transition to Dormant.
    pub fn check_timeout(&self) -> bool {
        if self.state != VoicePresenceState::Listening {
            return false;
        }
        self.last_activity.elapsed().as_secs() >= self.active_timeout_secs
    }

    /// EC-17.3: Queue a message that was interrupted by barge-in.
    pub fn queue_interrupted(&mut self, msg: String) {
        if self.interrupted_messages.len() < constants::INTERRUPTED_MSG_MAX {
            self.interrupted_messages.push(msg);
        }
    }

    /// EC-17.3: Drain all interrupted messages for replay.
    pub fn drain_interrupted(&mut self) -> Vec<String> {
        std::mem::take(&mut self.interrupted_messages)
    }

    /// Whether the session is active (not Dormant).
    pub fn is_active(&self) -> bool {
        self.state != VoicePresenceState::Dormant
    }
}

impl Default for VoiceSession {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_transitions() {
        let mut s = VoiceSession::new();
        assert_eq!(s.state, VoicePresenceState::Dormant);
        s.activate();
        assert_eq!(s.state, VoicePresenceState::Listening);
        s.speech_complete();
        assert_eq!(s.state, VoicePresenceState::Processing);
        s.response_ready();
        assert_eq!(s.state, VoicePresenceState::Speaking);
        s.done_speaking();
        assert_eq!(s.state, VoicePresenceState::Listening);
        s.go_dormant();
        assert_eq!(s.state, VoicePresenceState::Dormant);
    }

    #[test]
    fn multi_turn_stays_listening() {
        let mut s = VoiceSession::new();
        s.activate();
        s.speech_complete();
        s.response_ready();
        s.done_speaking();
        // After done_speaking, should be Listening (not Dormant)
        assert_eq!(s.state, VoicePresenceState::Listening);
        assert!(s.is_active());
    }

    #[test]
    fn turn_count_increments() {
        let mut s = VoiceSession::new();
        s.activate();
        assert_eq!(s.turn_count, 0);
        s.speech_complete();
        assert_eq!(s.turn_count, 1);
        s.response_ready();
        s.done_speaking();
        s.speech_complete();
        assert_eq!(s.turn_count, 2);
    }

    #[test]
    fn interrupted_messages_queued() {
        let mut s = VoiceSession::new();
        s.queue_interrupted("Security alert detected".into());
        s.queue_interrupted("Server high CPU".into());
        assert_eq!(s.interrupted_messages.len(), 2);
        let msgs = s.drain_interrupted();
        assert_eq!(msgs.len(), 2);
        assert!(s.interrupted_messages.is_empty());
    }

    #[test]
    fn timeout_only_in_listening() {
        let mut s = VoiceSession::new();
        // Dormant — no timeout
        assert!(!s.check_timeout());
        s.activate();
        s.speech_complete();
        // Processing — no timeout
        assert!(!s.check_timeout());
    }
}
