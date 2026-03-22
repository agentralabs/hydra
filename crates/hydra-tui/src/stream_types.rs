//! Stream item types — the atoms of the conversation stream.
//!
//! Separated from stream.rs to stay under the 400-line limit.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dot::DotKind;

/// Status of a companion-initiated task visible in the stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanionStatus {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Complete,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
}

impl std::fmt::Display for CompanionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Complete => write!(f, "complete"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Priority level for briefing items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BriefingPriority {
    /// Low priority — informational only.
    Low,
    /// Normal priority — worth noting.
    Normal,
    /// High priority — requires attention.
    High,
    /// Urgent — requires immediate attention.
    Urgent,
}

impl std::fmt::Display for BriefingPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

/// A single item in the conversation stream.
#[derive(Debug, Clone)]
pub enum StreamItem {
    /// A message from the user.
    UserMessage {
        /// Unique ID for this item.
        id: Uuid,
        /// The message text.
        text: String,
        /// When it was sent.
        timestamp: DateTime<Utc>,
    },

    /// Text from the assistant.
    AssistantText {
        /// Unique ID for this item.
        id: Uuid,
        /// The text content.
        text: String,
        /// When it was generated.
        timestamp: DateTime<Utc>,
    },

    /// A tool status dot.
    ToolDot {
        /// Unique ID for this item.
        id: Uuid,
        /// Tool name.
        tool_name: String,
        /// The dot kind.
        kind: DotKind,
        /// When the tool was invoked.
        timestamp: DateTime<Utc>,
    },

    /// A connector line between tool dots.
    ToolConnector {
        /// Unique ID for this item.
        id: Uuid,
        /// Label for the connector.
        label: String,
    },

    /// A truncation marker.
    Truncation {
        /// Unique ID for this item.
        id: Uuid,
        /// How many characters were truncated.
        chars_truncated: usize,
    },

    /// A belief citation reference.
    BeliefCitation {
        /// Unique ID for this item.
        id: Uuid,
        /// The belief being cited.
        belief: String,
        /// Confidence level (0.0 to 1.0).
        confidence: f64,
    },

    /// A companion task visible in the stream.
    CompanionTask {
        /// Unique ID for this item.
        id: Uuid,
        /// Task description.
        description: String,
        /// Current status.
        status: CompanionStatus,
        /// When the task was created.
        timestamp: DateTime<Utc>,
    },

    /// A briefing item from the ambient/dream threads.
    BriefingItem {
        /// Unique ID for this item.
        id: Uuid,
        /// Briefing content.
        content: String,
        /// Priority level.
        priority: BriefingPriority,
        /// When the briefing was generated.
        timestamp: DateTime<Utc>,
    },

    /// A notification from the dream thread.
    DreamNotification {
        /// Unique ID for this item.
        id: Uuid,
        /// Dream content summary.
        content: String,
        /// When the dream occurred.
        timestamp: DateTime<Utc>,
    },

    /// A system notification.
    SystemNotification {
        /// Unique ID for this item.
        id: Uuid,
        /// Notification content.
        content: String,
        /// When the notification was generated.
        timestamp: DateTime<Utc>,
    },

    /// A blank line separator.
    Blank,
}

impl StreamItem {
    /// Return the unique ID for this item, if it has one.
    pub fn id(&self) -> Option<Uuid> {
        match self {
            Self::UserMessage { id, .. }
            | Self::AssistantText { id, .. }
            | Self::ToolDot { id, .. }
            | Self::ToolConnector { id, .. }
            | Self::Truncation { id, .. }
            | Self::BeliefCitation { id, .. }
            | Self::CompanionTask { id, .. }
            | Self::BriefingItem { id, .. }
            | Self::DreamNotification { id, .. }
            | Self::SystemNotification { id, .. } => Some(*id),
            Self::Blank => None,
        }
    }

    /// Return a short label describing the item kind.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::UserMessage { .. } => "user",
            Self::AssistantText { .. } => "assistant",
            Self::ToolDot { .. } => "tool-dot",
            Self::ToolConnector { .. } => "tool-connector",
            Self::Truncation { .. } => "truncation",
            Self::BeliefCitation { .. } => "belief-citation",
            Self::CompanionTask { .. } => "companion-task",
            Self::BriefingItem { .. } => "briefing",
            Self::DreamNotification { .. } => "dream",
            Self::SystemNotification { .. } => "system",
            Self::Blank => "blank",
        }
    }
}
