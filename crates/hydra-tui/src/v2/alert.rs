//! Alert system — 3 levels per spec Part Seven.
//! L1: Stream notification (normal). L2: Status bar flash. L3: RED + TTS + bell.

use chrono::{DateTime, Utc};

/// Alert severity level.
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    /// Normal stream notification.
    Stream,
    /// Status bar flashes AMBER for 3 seconds.
    Frame,
    /// RED border + TTS voice + bell.
    Emergency,
}

/// Active alert state.
#[derive(Debug, Clone)]
pub struct AlertState {
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub dismissed: bool,
}

impl AlertState {
    pub fn frame(message: impl Into<String>) -> Self {
        Self { level: AlertLevel::Frame, message: message.into(), timestamp: Utc::now(), dismissed: false }
    }

    pub fn emergency(message: impl Into<String>) -> Self {
        Self { level: AlertLevel::Emergency, message: message.into(), timestamp: Utc::now(), dismissed: false }
    }

    /// Whether the alert has expired (3s for Frame, 10s for Emergency).
    pub fn is_expired(&self) -> bool {
        let age = (Utc::now() - self.timestamp).num_seconds();
        match self.level {
            AlertLevel::Stream => true, // instant
            AlertLevel::Frame => age > 3,
            AlertLevel::Emergency => age > 10 || self.dismissed,
        }
    }

    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }
}

/// Classify an enrichment key into an alert level.
pub fn classify_alert(key: &str, value: &str) -> Option<AlertState> {
    match key {
        "security.threat" => Some(AlertState::emergency(format!("SECURITY: {}", truncate(value, 60)))),
        "judgment" if value.contains("REFUSED") => Some(AlertState::frame(format!("REFUSED: {}", truncate(value, 60)))),
        "judgment" if value.contains("APPROVAL") => Some(AlertState::frame(format!("NEEDS APPROVAL: {}", truncate(value, 60)))),
        _ => None,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}...", &s[..max.min(s.len())]) } else { s.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_not_expired_immediately() {
        let a = AlertState::emergency("test");
        assert!(!a.is_expired());
    }

    #[test]
    fn classify_threat() {
        let a = classify_alert("security.threat", "prompt injection");
        assert!(a.is_some());
        assert_eq!(a.unwrap().level, AlertLevel::Emergency);
    }

    #[test]
    fn classify_normal_key() {
        assert!(classify_alert("memory.context", "data").is_none());
    }
}
