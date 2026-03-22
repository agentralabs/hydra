//! TraceEvent — one event in the execution timeline.
//! Built from receipts. Ordered chronologically.

use serde::{Deserialize, Serialize};

/// The type of event in an execution trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventKind {
    /// Task was initiated.
    TaskStarted { intent: String },
    /// An approach was attempted.
    ApproachAttempted { approach: String },
    /// An obstacle was encountered.
    ObstacleEncountered { obstacle: String },
    /// An approach succeeded.
    ApproachSucceeded { approach: String },
    /// Rerouting to next approach.
    Rerouting { from: String, to: String },
    /// Escalated to fleet agent.
    AgentEscalated { agent_type: String },
    /// Task completed successfully.
    TaskCompleted { duration_total_ms: u64 },
    /// Task hard denied.
    TaskHardDenied { evidence: String },
    /// Automation observation recorded.
    AutomationObserved { action_id: String },
}

impl EventKind {
    /// Short label for the event kind.
    pub fn label(&self) -> &'static str {
        match self {
            Self::TaskStarted { .. } => "started",
            Self::ApproachAttempted { .. } => "attempt",
            Self::ObstacleEncountered { .. } => "blocked",
            Self::ApproachSucceeded { .. } => "succeeded",
            Self::Rerouting { .. } => "rerouting",
            Self::AgentEscalated { .. } => "escalated",
            Self::TaskCompleted { .. } => "completed",
            Self::TaskHardDenied { .. } => "hard-denied",
            Self::AutomationObserved { .. } => "automation",
        }
    }

    /// Whether this event terminates an execution trace.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::TaskCompleted { .. } | Self::TaskHardDenied { .. }
        )
    }

    /// One-line human description.
    pub fn describe(&self) -> String {
        match self {
            Self::TaskStarted { intent } => {
                format!("Task started: \"{}\"", &intent[..intent.len().min(60)])
            }
            Self::ApproachAttempted { approach } => {
                format!("Attempted: {}", approach)
            }
            Self::ObstacleEncountered { obstacle } => {
                format!("Blocked: {}", &obstacle[..obstacle.len().min(80)])
            }
            Self::ApproachSucceeded { approach } => {
                format!("Succeeded via: {}", approach)
            }
            Self::Rerouting { from, to } => {
                format!("Rerouting {} → {}", from, to)
            }
            Self::AgentEscalated { agent_type } => {
                format!("Escalated to {} agent", agent_type)
            }
            Self::TaskCompleted { duration_total_ms } => {
                format!(
                    "Completed in {:.1}s",
                    *duration_total_ms as f64 / 1000.0
                )
            }
            Self::TaskHardDenied { evidence } => {
                format!("Hard denied: {}", &evidence[..evidence.len().min(80)])
            }
            Self::AutomationObserved { action_id } => {
                format!("Automation observed: {}", action_id)
            }
        }
    }
}

/// One event in the execution timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Unique event ID.
    pub id: String,
    /// The task this event belongs to.
    pub task_id: String,
    /// The kind of event.
    pub kind: EventKind,
    /// The receipt ID associated with this event.
    pub receipt_id: String,
    /// When the event occurred.
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    /// Duration of this event in milliseconds.
    pub duration_ms: u64,
}

impl TraceEvent {
    /// Create a new trace event with the current timestamp.
    pub fn new(
        task_id: &str,
        kind: EventKind,
        receipt_id: &str,
        duration_ms: u64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            kind,
            receipt_id: receipt_id.to_string(),
            occurred_at: chrono::Utc::now(),
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_is_terminal() {
        let k = EventKind::TaskCompleted {
            duration_total_ms: 1000,
        };
        assert!(k.is_terminal());
    }

    #[test]
    fn attempt_not_terminal() {
        let k = EventKind::ApproachAttempted {
            approach: "direct".into(),
        };
        assert!(!k.is_terminal());
    }

    #[test]
    fn describe_non_empty_for_all_kinds() {
        let kinds = vec![
            EventKind::TaskStarted {
                intent: "deploy".into(),
            },
            EventKind::ApproachAttempted {
                approach: "direct".into(),
            },
            EventKind::ObstacleEncountered {
                obstacle: "auth failed".into(),
            },
            EventKind::ApproachSucceeded {
                approach: "alt".into(),
            },
            EventKind::Rerouting {
                from: "direct".into(),
                to: "alt".into(),
            },
            EventKind::TaskCompleted {
                duration_total_ms: 2000,
            },
            EventKind::TaskHardDenied {
                evidence: "401".into(),
            },
        ];
        for k in kinds {
            assert!(!k.describe().is_empty());
        }
    }
}
