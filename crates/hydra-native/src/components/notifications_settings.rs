//! Push notification settings component data (Step 4.10).
//!
//! Manages device registration, push providers, urgency filtering,
//! and trigger-based notification dispatch logic.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported push notification transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PushProvider {
    WebPush,
    Ntfy,
    Telegram,
    Email,
}

/// Urgency classification for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationUrgency {
    Low,
    Normal,
    High,
    Critical,
}

/// Events that can trigger a push notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationTrigger {
    ApprovalRequired,
    TaskCompleted,
    TaskFailed,
    KillSwitchActivated,
    ProactiveInsight,
}

impl NotificationTrigger {
    /// Inherent urgency level for this trigger type.
    pub fn default_urgency(&self) -> NotificationUrgency {
        match self {
            Self::KillSwitchActivated => NotificationUrgency::Critical,
            Self::ApprovalRequired => NotificationUrgency::High,
            Self::TaskFailed => NotificationUrgency::High,
            Self::TaskCompleted => NotificationUrgency::Normal,
            Self::ProactiveInsight => NotificationUrgency::Low,
        }
    }
}

/// A registered device that receives notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub provider: PushProvider,
    pub push_token: String,
    pub last_seen: Option<String>,
    pub urgency_filter: Vec<NotificationUrgency>,
}

/// Top-level notification settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub provider: PushProvider,
    pub devices: Vec<DeviceInfo>,
    pub ntfy_topic_id: Option<String>,
    pub telegram_bot_linked: bool,
    pub telegram_chat_id: Option<String>,
}

impl NotificationSettings {
    /// Create notification settings, disabled by default.
    pub fn new() -> Self {
        Self {
            enabled: false,
            provider: PushProvider::WebPush,
            devices: Vec::new(),
            ntfy_topic_id: None,
            telegram_bot_linked: false,
            telegram_chat_id: None,
        }
    }

    /// Register a new device.
    pub fn add_device(&mut self, device: DeviceInfo) {
        self.devices.push(device);
    }

    /// Remove a device by name. Returns true if a device was removed.
    pub fn remove_device(&mut self, name: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.name != name);
        self.devices.len() < before
    }

    /// Generate a UUID-based ntfy topic and store it.
    pub fn generate_ntfy_topic(&mut self) -> String {
        let topic = format!("hydra-{}", Uuid::new_v4());
        self.ntfy_topic_id = Some(topic.clone());
        topic
    }

    /// Determine whether a notification should fire for the given trigger
    /// and urgency, considering global enable state and device filters.
    pub fn should_notify(
        &self,
        _trigger: &NotificationTrigger,
        urgency: &NotificationUrgency,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        if self.devices.is_empty() {
            return false;
        }
        // At least one device must accept the urgency level.
        self.devices
            .iter()
            .any(|d| d.urgency_filter.contains(urgency))
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device(name: &str, urgencies: Vec<NotificationUrgency>) -> DeviceInfo {
        DeviceInfo {
            name: name.to_string(),
            provider: PushProvider::WebPush,
            push_token: "tok_test".to_string(),
            last_seen: None,
            urgency_filter: urgencies,
        }
    }

    #[test]
    fn test_new_is_disabled() {
        let s = NotificationSettings::new();
        assert!(!s.enabled);
        assert!(s.devices.is_empty());
        assert_eq!(s.provider, PushProvider::WebPush);
        assert!(s.ntfy_topic_id.is_none());
        assert!(!s.telegram_bot_linked);
    }

    #[test]
    fn test_add_and_remove_device() {
        let mut s = NotificationSettings::new();
        s.add_device(sample_device("phone", vec![NotificationUrgency::High]));
        s.add_device(sample_device("laptop", vec![NotificationUrgency::Normal]));
        assert_eq!(s.devices.len(), 2);

        assert!(s.remove_device("phone"));
        assert_eq!(s.devices.len(), 1);
        assert_eq!(s.devices[0].name, "laptop");

        assert!(!s.remove_device("nonexistent"));
        assert_eq!(s.devices.len(), 1);
    }

    #[test]
    fn test_generate_ntfy_topic() {
        let mut s = NotificationSettings::new();
        assert!(s.ntfy_topic_id.is_none());
        let topic = s.generate_ntfy_topic();
        assert!(topic.starts_with("hydra-"));
        assert_eq!(s.ntfy_topic_id, Some(topic.clone()));

        // Generating again produces a new topic.
        let topic2 = s.generate_ntfy_topic();
        assert_ne!(topic, topic2);
    }

    #[test]
    fn test_should_notify_disabled() {
        let mut s = NotificationSettings::new();
        s.add_device(sample_device(
            "phone",
            vec![NotificationUrgency::Critical],
        ));
        // Not enabled — should never notify.
        assert!(!s.should_notify(
            &NotificationTrigger::KillSwitchActivated,
            &NotificationUrgency::Critical,
        ));
    }

    #[test]
    fn test_should_notify_matching_urgency() {
        let mut s = NotificationSettings::new();
        s.enabled = true;
        s.add_device(sample_device(
            "phone",
            vec![NotificationUrgency::High, NotificationUrgency::Critical],
        ));

        assert!(s.should_notify(
            &NotificationTrigger::ApprovalRequired,
            &NotificationUrgency::High,
        ));
        assert!(!s.should_notify(
            &NotificationTrigger::TaskCompleted,
            &NotificationUrgency::Low,
        ));
    }

    #[test]
    fn test_should_notify_no_devices() {
        let mut s = NotificationSettings::new();
        s.enabled = true;
        assert!(!s.should_notify(
            &NotificationTrigger::TaskFailed,
            &NotificationUrgency::High,
        ));
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut s = NotificationSettings::new();
        s.enabled = true;
        s.provider = PushProvider::Ntfy;
        s.generate_ntfy_topic();
        s.add_device(sample_device(
            "tablet",
            vec![NotificationUrgency::Normal, NotificationUrgency::High],
        ));

        let json = serde_json::to_string(&s).expect("serialize");
        let restored: NotificationSettings =
            serde_json::from_str(&json).expect("deserialize");
        assert!(restored.enabled);
        assert_eq!(restored.provider, PushProvider::Ntfy);
        assert_eq!(restored.devices.len(), 1);
        assert_eq!(restored.devices[0].name, "tablet");
        assert!(restored.ntfy_topic_id.is_some());
    }

    #[test]
    fn test_trigger_default_urgency() {
        assert_eq!(
            NotificationTrigger::KillSwitchActivated.default_urgency(),
            NotificationUrgency::Critical,
        );
        assert_eq!(
            NotificationTrigger::ProactiveInsight.default_urgency(),
            NotificationUrgency::Low,
        );
    }
}
