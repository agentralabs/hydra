use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Urgency level for a notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationUrgency {
    Low,
    Normal,
    High,
}

/// Action associated with a notification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAction {
    OpenApp,
    ApproveRun(String),
    Dismiss,
}

/// A notification in the Hydra system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub urgency: NotificationUrgency,
    pub action: Option<NotificationAction>,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}
