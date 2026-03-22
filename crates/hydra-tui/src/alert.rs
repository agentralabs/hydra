//! Alert system — visual + audio alerts for critical events.
//!
//! Three levels:
//! - Stream: normal stream item (default)
//! - Frame: amber status bar flash + alert bar
//! - Emergency: RED border flash + alert bar + voice + terminal bell

use std::time::{Duration, Instant};

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    /// Normal stream notification. No special visual treatment.
    Stream,
    /// Notable event. Status bar flashes amber, alert bar shown.
    Frame,
    /// Critical event. RED border flash, voice alert, terminal bell.
    Emergency,
}

/// A single alert to display.
#[derive(Debug, Clone)]
pub struct Alert {
    /// Severity level.
    pub level: AlertLevel,
    /// Alert message text.
    pub message: String,
    /// When the alert was created.
    pub created_at: Instant,
    /// Whether the user has dismissed it.
    pub dismissed: bool,
}

/// Duration before alerts auto-dismiss.
const ALERT_AUTO_DISMISS_MS: u64 = 10_000;
/// Duration for emergency border flash.
const EMERGENCY_FLASH_MS: u64 = 3_000;
/// Duration for frame status bar flash.
const FRAME_FLASH_MS: u64 = 2_000;

impl Alert {
    /// Create a new alert.
    pub fn new(level: AlertLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            created_at: Instant::now(),
            dismissed: false,
        }
    }

    /// Whether this alert should still be shown.
    pub fn is_active(&self) -> bool {
        if self.dismissed {
            return false;
        }
        self.created_at.elapsed() < Duration::from_millis(ALERT_AUTO_DISMISS_MS)
    }

    /// Whether the border/status flash should be active.
    pub fn is_flashing(&self) -> bool {
        if self.dismissed {
            return false;
        }
        let flash_ms = match self.level {
            AlertLevel::Emergency => EMERGENCY_FLASH_MS,
            AlertLevel::Frame => FRAME_FLASH_MS,
            AlertLevel::Stream => 0,
        };
        self.created_at.elapsed() < Duration::from_millis(flash_ms)
    }
}

/// Manages the current alert state.
#[derive(Debug, Clone)]
pub struct AlertManager {
    /// Current active alert (only one at a time — highest priority wins).
    current: Option<Alert>,
    /// Whether terminal bell has been fired for current alert.
    bell_fired: bool,
}

impl AlertManager {
    /// Create a new alert manager.
    pub fn new() -> Self {
        Self {
            current: None,
            bell_fired: false,
        }
    }

    /// Push a new alert. Higher level replaces lower level.
    pub fn push(&mut self, alert: Alert) {
        let should_replace = match &self.current {
            None => true,
            Some(existing) => {
                !existing.is_active() || alert.level >= existing.level
            }
        };
        if should_replace {
            self.bell_fired = false;
            self.current = Some(alert);
        }
    }

    /// Dismiss the current alert (user pressed Escape).
    pub fn dismiss(&mut self) {
        if let Some(alert) = &mut self.current {
            alert.dismissed = true;
        }
    }

    /// Get the current active alert (if any).
    pub fn current(&self) -> Option<&Alert> {
        self.current.as_ref().filter(|a| a.is_active())
    }

    /// Whether the border should flash (emergency alerts).
    pub fn is_border_flashing(&self) -> bool {
        self.current
            .as_ref()
            .map(|a| a.level == AlertLevel::Emergency && a.is_flashing())
            .unwrap_or(false)
    }

    /// Whether the status bar should flash (frame alerts).
    pub fn is_status_flashing(&self) -> bool {
        self.current
            .as_ref()
            .map(|a| a.level >= AlertLevel::Frame && a.is_flashing())
            .unwrap_or(false)
    }

    /// Whether terminal bell should fire (once per emergency).
    pub fn should_bell(&mut self) -> bool {
        if self.bell_fired {
            return false;
        }
        if let Some(a) = &self.current {
            if a.level == AlertLevel::Emergency && a.is_active() {
                self.bell_fired = true;
                return true;
            }
        }
        false
    }

    /// Get voice text for emergency alerts (if applicable).
    pub fn voice_text(&self) -> Option<&str> {
        self.current.as_ref().and_then(|a| {
            if a.level == AlertLevel::Emergency && a.is_active() {
                Some(a.message.as_str())
            } else {
                None
            }
        })
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Classify a kernel enrichment key into an alert level.
pub fn classify_enrichment(key: &str, value: &str) -> AlertLevel {
    match key {
        "security.threat" => AlertLevel::Emergency,
        "surprise" if value.contains("constitutional") => AlertLevel::Emergency,
        "redteam" if value.contains("NO-GO") => AlertLevel::Emergency,
        "redteam" if value.contains("threat") => AlertLevel::Frame,
        "oracle" if value.contains("adverse") => AlertLevel::Frame,
        _ => AlertLevel::Stream,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_auto_dismisses() {
        let alert = Alert::new(AlertLevel::Stream, "test");
        assert!(alert.is_active());
        // Can't easily test time-based dismissal in unit test
    }

    #[test]
    fn emergency_has_voice() {
        let mut mgr = AlertManager::new();
        mgr.push(Alert::new(AlertLevel::Emergency, "Security alert"));
        assert!(mgr.voice_text().is_some());
        assert!(mgr.should_bell());
        assert!(!mgr.should_bell()); // only fires once
    }

    #[test]
    fn dismiss_clears_alert() {
        let mut mgr = AlertManager::new();
        mgr.push(Alert::new(AlertLevel::Frame, "Warning"));
        assert!(mgr.current().is_some());
        mgr.dismiss();
        assert!(mgr.current().is_none());
    }

    #[test]
    fn higher_level_replaces_lower() {
        let mut mgr = AlertManager::new();
        mgr.push(Alert::new(AlertLevel::Frame, "warning"));
        mgr.push(Alert::new(AlertLevel::Emergency, "critical"));
        assert_eq!(mgr.current().unwrap().level, AlertLevel::Emergency);
    }
}
