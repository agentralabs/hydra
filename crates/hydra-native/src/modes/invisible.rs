//! Invisible mode — system tray / background operation.

use serde::{Deserialize, Serialize};

/// Icon state shown in the system tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IconState {
    Idle,
    Active,
    Error,
}

/// Notification urgency level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationUrgency {
    Low,
    Normal,
    High,
}

/// A desktop notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub urgency: NotificationUrgency,
}

/// System-tray / invisible mode: Hydra runs in the background.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvisibleMode {
    pub tray_icon_state: IconState,
    pub notification_enabled: bool,
}

impl InvisibleMode {
    pub fn new() -> Self {
        Self {
            tray_icon_state: IconState::Idle,
            notification_enabled: true,
        }
    }

    /// Build a notification to show to the user.
    pub fn show_notification(title: &str, body: &str) -> Notification {
        Notification {
            title: title.to_string(),
            body: body.to_string(),
            urgency: NotificationUrgency::Normal,
        }
    }

    /// Tooltip text for the tray icon based on active run count.
    pub fn status_tooltip(active_runs: usize) -> String {
        if active_runs == 0 {
            "All good".to_string()
        } else if active_runs == 1 {
            "1 task running".to_string()
        } else {
            format!("{} tasks running", active_runs)
        }
    }
}

impl Default for InvisibleMode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let m = InvisibleMode::new();
        assert_eq!(m.tray_icon_state, IconState::Idle);
        assert!(m.notification_enabled);
    }

    #[test]
    fn test_show_notification() {
        let n = InvisibleMode::show_notification("Done", "Task finished");
        assert_eq!(n.title, "Done");
        assert_eq!(n.body, "Task finished");
        assert_eq!(n.urgency, NotificationUrgency::Normal);
    }

    #[test]
    fn test_status_tooltip() {
        assert_eq!(InvisibleMode::status_tooltip(0), "All good");
        assert_eq!(InvisibleMode::status_tooltip(1), "1 task running");
        assert_eq!(InvisibleMode::status_tooltip(2), "2 tasks running");
        assert_eq!(InvisibleMode::status_tooltip(5), "5 tasks running");
    }

    #[test]
    fn test_serialization() {
        let m = InvisibleMode::new();
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("idle"));
    }

    #[test]
    fn test_notification_urgency_serialization() {
        let urgencies = [NotificationUrgency::Low, NotificationUrgency::Normal, NotificationUrgency::High];
        for u in &urgencies {
            let json = serde_json::to_string(u).unwrap();
            let back: NotificationUrgency = serde_json::from_str(&json).unwrap();
            assert_eq!(*u, back);
        }
    }

    #[test]
    fn test_icon_state_serialization() {
        let states = [IconState::Idle, IconState::Active, IconState::Error];
        for s in &states {
            let json = serde_json::to_string(s).unwrap();
            let back: IconState = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn test_deserialization_roundtrip() {
        let m = InvisibleMode::new();
        let json = serde_json::to_string(&m).unwrap();
        let back: InvisibleMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tray_icon_state, IconState::Idle);
        assert!(back.notification_enabled);
    }
}
