//! O19 Presence Detection — tracks user presence via motion detection.
//! Privacy: no face recognition, no identity — just present/away/idle.
//! Camera off by default (EC-19.10). Triggers briefings on return.

use std::time::Instant;
use crate::gesture::{GestureClassifier, GestureCommand, map_command};
use crate::webcam;

/// User presence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    Disabled,
    Present,
    Idle,
    Away,
}

impl PresenceState {
    pub fn label(&self) -> &'static str {
        match self { Self::Disabled => "disabled", Self::Present => "present",
            Self::Idle => "idle", Self::Away => "away" }
    }
    pub fn status_icon(&self) -> &'static str {
        match self { Self::Disabled => "", Self::Present => "CAM",
            Self::Idle => "IDLE", Self::Away => "AWAY" }
    }
}

/// Presence engine — manages state transitions, gesture detection, polling.
pub struct PresenceEngine {
    enabled: bool,
    state: PresenceState,
    classifier: GestureClassifier,
    last_motion: Instant,
    idle_timeout_secs: u64,
    away_timeout_secs: u64,
    poll_interval_secs: u64,
    last_poll: Instant,
    prev_digest: Option<webcam::FrameDigest>,
    #[allow(dead_code)]
    last_command: Option<GestureCommand>,
}

impl PresenceEngine {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            enabled: false, state: PresenceState::Disabled,
            classifier: GestureClassifier::new(),
            last_motion: now, idle_timeout_secs: 30, away_timeout_secs: 300,
            poll_interval_secs: 2, last_poll: now, prev_digest: None,
            last_command: None,
        }
    }

    /// Enable camera presence detection. Fails if no webcam (EC-19.1).
    pub fn enable(&mut self) -> Result<(), crate::errors::DesktopError> {
        if !webcam::webcam_available() {
            return Err(crate::errors::DesktopError::CameraError("No webcam available".into()));
        }
        self.enabled = true;
        self.state = PresenceState::Present;
        self.last_motion = Instant::now();
        eprintln!("hydra-presence: camera enabled");
        Ok(())
    }

    /// Disable camera. Stops all capture immediately.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.state = PresenceState::Disabled;
        self.prev_digest = None;
        self.classifier.clear();
        eprintln!("hydra-presence: camera disabled");
    }

    /// Poll for presence update. Only captures every poll_interval_secs.
    /// Returns (state_changed, gesture_command).
    pub fn poll(&mut self) -> (bool, Option<GestureCommand>) {
        if !self.enabled { return (false, None); }
        if self.last_poll.elapsed().as_secs() < self.poll_interval_secs { return (false, None); }
        self.last_poll = Instant::now();

        // Capture frame
        let frame = match webcam::capture_frame() {
            Ok(bytes) => webcam::FrameDigest::from_rgb(320, 240, &bytes),
            Err(e) => { eprintln!("hydra-presence: capture failed: {e}"); return (false, None); }
        };

        // Compute motion
        let motion = if let Some(ref prev) = self.prev_digest {
            prev.motion_score(&frame)
        } else { 0.0 };
        self.prev_digest = Some(frame);

        // Classify gesture
        let gesture = self.classifier.feed(motion);
        let cmd = map_command(&gesture);
        let cmd_result = if cmd != GestureCommand::None { Some(cmd) } else { None };

        // Update presence state
        let old_state = self.state;
        let elapsed = self.last_motion.elapsed().as_secs();
        if motion > 0.02 {
            self.state = PresenceState::Present;
            self.last_motion = Instant::now();
        } else if elapsed >= self.away_timeout_secs {
            self.state = PresenceState::Away;
        } else if elapsed >= self.idle_timeout_secs {
            self.state = PresenceState::Idle;
        }

        let changed = self.state != old_state;
        if changed { eprintln!("hydra-presence: {} → {}", old_state.label(), self.state.label()); }
        (changed, cmd_result)
    }

    pub fn state(&self) -> &PresenceState { &self.state }
    pub fn enabled(&self) -> bool { self.enabled }

    /// Whether alerts should be suppressed (user away).
    pub fn should_suppress_alerts(&self) -> bool {
        self.state == PresenceState::Away
    }
}

impl Default for PresenceEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_disabled() {
        let e = PresenceEngine::new();
        assert_eq!(*e.state(), PresenceState::Disabled);
        assert!(!e.enabled());
    }

    #[test]
    fn state_labels() {
        assert_eq!(PresenceState::Present.label(), "present");
        assert_eq!(PresenceState::Idle.label(), "idle");
        assert_eq!(PresenceState::Away.label(), "away");
        assert_eq!(PresenceState::Disabled.status_icon(), "");
        assert_eq!(PresenceState::Present.status_icon(), "CAM");
    }

    #[test]
    fn poll_when_disabled() {
        let mut e = PresenceEngine::new();
        let (changed, cmd) = e.poll();
        assert!(!changed);
        assert!(cmd.is_none());
    }

    #[test]
    fn suppress_when_away() {
        let mut e = PresenceEngine::new();
        e.state = PresenceState::Away;
        assert!(e.should_suppress_alerts());
        e.state = PresenceState::Present;
        assert!(!e.should_suppress_alerts());
    }
}
