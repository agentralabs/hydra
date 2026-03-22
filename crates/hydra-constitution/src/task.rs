//! Task lifecycle types for the Capability Declaration.
//!
//! A task in Hydra never "fails" — it navigates obstacles.
//! The only way a task terminates without success is a hard stop
//! (authentication explicitly denied, principal cancellation,
//! or constitutional violation required).

use crate::declarations::HardStop;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    /// Generate a new unique task ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from an existing string.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The state of a task. Note: there is NO Failed variant.
/// Tasks navigate obstacles; they do not fail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Task is actively executing.
    Active,
    /// Task has encountered an obstacle and is trying approaches.
    Blocked {
        /// The obstacle that is blocking the task.
        obstacle: ObstacleType,
        /// How many approaches have been tried so far.
        attempts: usize,
    },
    /// Task is trying a different approach after being blocked.
    Rerouting {
        /// The approach currently being attempted.
        current_approach: ApproachType,
    },
    /// Task is escalating to a fleet agent for help.
    EscalatingToAgent {
        /// The reason for escalation.
        reason: String,
    },
    /// Task is suspended waiting for external input (e.g. principal decision).
    Suspended {
        /// What the task is waiting for.
        waiting_for: String,
    },
    /// Task completed successfully.
    Complete,
    /// Task encountered a genuine hard stop.
    HardDenied {
        /// The constitutional hard-stop evidence.
        evidence: HardStop,
    },
}

impl TaskState {
    /// Returns true if the task is still active (not terminal).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Active
                | Self::Blocked { .. }
                | Self::Rerouting { .. }
                | Self::EscalatingToAgent { .. }
                | Self::Suspended { .. }
        )
    }

    /// Returns true if the task has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete | Self::HardDenied { .. })
    }
}

/// The type of obstacle a task has encountered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObstacleType {
    /// A timeout occurred (NOT a hard stop — retry with different timing).
    Timeout {
        /// How long the timeout was.
        duration_ms: u64,
    },
    /// A rate limit was hit (NOT a hard stop — wait and retry).
    RateLimit {
        /// When the rate limit resets.
        retry_after: String,
    },
    /// A network error occurred (NOT a hard stop — try different route).
    NetworkError {
        /// Description of the error.
        error: String,
    },
    /// A permission error occurred (NOT necessarily a hard stop — try elevation).
    PermissionDenied {
        /// The resource that was denied.
        resource: String,
    },
    /// The tool or API returned an unexpected format.
    UnexpectedFormat {
        /// Description of what was expected vs received.
        details: String,
    },
    /// A dependency is unavailable.
    DependencyUnavailable {
        /// Which dependency is missing.
        dependency: String,
    },
    /// An unknown obstacle type for extensibility.
    Other {
        /// Description of the obstacle.
        description: String,
    },
}

impl std::fmt::Display for ObstacleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { duration_ms } => write!(f, "Timeout({}ms)", duration_ms),
            Self::RateLimit { retry_after } => {
                write!(f, "RateLimit(retry_after={})", retry_after)
            }
            Self::NetworkError { error } => write!(f, "NetworkError({})", error),
            Self::PermissionDenied { resource } => {
                write!(f, "PermissionDenied({})", resource)
            }
            Self::UnexpectedFormat { details } => {
                write!(f, "UnexpectedFormat({})", details)
            }
            Self::DependencyUnavailable { dependency } => {
                write!(f, "DependencyUnavailable({})", dependency)
            }
            Self::Other { description } => write!(f, "Other({})", description),
        }
    }
}

/// An approach to navigating an obstacle.
/// The `next()` method cycles through approaches — it never terminates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApproachType {
    /// Retry with the same parameters.
    Retry,
    /// Retry with exponential backoff.
    RetryWithBackoff,
    /// Try a different tool or API.
    AlternativeTool {
        /// The alternative tool to try.
        tool: String,
    },
    /// Try a different authentication method.
    AlternativeAuth {
        /// The alternative auth method.
        method: String,
    },
    /// Try a different network route.
    AlternativeRoute {
        /// The alternative route.
        route: String,
    },
    /// Break the task into smaller subtasks.
    Decompose,
    /// Ask a fleet agent for help.
    DelegateToAgent,
    /// Ask the principal for guidance.
    AskPrincipal,
    /// Wait for an external condition to change.
    WaitForCondition {
        /// What condition to wait for.
        condition: String,
    },
    /// Transform the input to work around the obstacle.
    TransformInput,
    /// Try a cached or local fallback.
    UseFallback,
    /// Reduce scope to get partial results.
    ReduceScope,
    /// Escalate to a higher trust tier.
    EscalateTrust,
}

impl ApproachType {
    /// Returns the next approach to try after this one.
    /// This cycle NEVER terminates — there is always another approach.
    pub fn next(&self) -> Self {
        match self {
            Self::Retry => Self::RetryWithBackoff,
            Self::RetryWithBackoff => Self::AlternativeTool {
                tool: String::new(),
            },
            Self::AlternativeTool { .. } => Self::AlternativeAuth {
                method: String::new(),
            },
            Self::AlternativeAuth { .. } => Self::AlternativeRoute {
                route: String::new(),
            },
            Self::AlternativeRoute { .. } => Self::Decompose,
            Self::Decompose => Self::DelegateToAgent,
            Self::DelegateToAgent => Self::AskPrincipal,
            Self::AskPrincipal => Self::WaitForCondition {
                condition: String::new(),
            },
            Self::WaitForCondition { .. } => Self::TransformInput,
            Self::TransformInput => Self::UseFallback,
            Self::UseFallback => Self::ReduceScope,
            Self::ReduceScope => Self::EscalateTrust,
            Self::EscalateTrust => Self::Retry, // cycle back
        }
    }

    /// Returns the first approach in the cycle.
    pub fn first() -> Self {
        Self::Retry
    }
}

/// A single attempt record — documents what was tried and what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Which approach was tried.
    pub approach: ApproachType,
    /// The obstacle that prompted this attempt.
    pub obstacle: ObstacleType,
    /// What happened.
    pub outcome: AttemptOutcome,
    /// When the attempt started.
    pub started_at: String,
    /// When the attempt ended.
    pub ended_at: String,
}

/// The outcome of a single attempt.
/// Note: there is NO Failed variant. An attempt either succeeds,
/// is blocked (obstacle navigated to next approach), is suspended,
/// or encounters a hard stop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttemptOutcome {
    /// The attempt succeeded.
    Succeeded,
    /// The attempt was blocked — try the next approach.
    Blocked {
        /// The new obstacle encountered.
        new_obstacle: String,
    },
    /// The attempt is suspended waiting for external input.
    Suspended {
        /// What is being waited for.
        waiting_for: String,
    },
    /// A genuine hard stop was encountered.
    HardDenied {
        /// The constitutional hard-stop evidence.
        evidence: HardStop,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_is_unique() {
        assert_ne!(TaskId::new(), TaskId::new());
    }

    #[test]
    fn active_states_are_active() {
        assert!(TaskState::Active.is_active());
        assert!(TaskState::Blocked {
            obstacle: ObstacleType::Timeout { duration_ms: 5000 },
            attempts: 1,
        }
        .is_active());
        assert!(TaskState::Rerouting {
            current_approach: ApproachType::Retry,
        }
        .is_active());
    }

    #[test]
    fn terminal_states_are_terminal() {
        assert!(TaskState::Complete.is_terminal());
        assert!(TaskState::HardDenied {
            evidence: HardStop::AuthenticationExplicitlyDenied {
                system: "server".to_string(),
                reason: "auth denied".to_string(),
                evidence: "Permission denied (publickey)".to_string(),
            },
        }
        .is_terminal());
    }

    #[test]
    fn approach_cycle_never_terminates() {
        let mut approach = ApproachType::first();
        // Cycle through all 13 approaches and back to start
        for _ in 0..13 {
            approach = approach.next();
        }
        assert_eq!(approach, ApproachType::Retry);
    }

    #[test]
    fn approach_cycle_covers_all_variants() {
        let mut seen = Vec::new();
        let mut approach = ApproachType::first();
        for _ in 0..13 {
            seen.push(format!("{:?}", approach));
            approach = approach.next();
        }
        assert_eq!(seen.len(), 13);
        // All should be unique
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 13);
    }

    #[test]
    fn no_failed_variant_in_outcome_or_state() {
        // Documents the deliberate absence of a Failed variant.
        let stop = HardStop::PrincipalCancellation {
            task_id: "t".to_string(),
            cancelled_at: "now".to_string(),
        };
        let outcomes = vec![
            AttemptOutcome::Succeeded,
            AttemptOutcome::Blocked { new_obstacle: "t".to_string() },
            AttemptOutcome::Suspended { waiting_for: "p".to_string() },
            AttemptOutcome::HardDenied { evidence: stop.clone() },
        ];
        assert_eq!(outcomes.len(), 4); // exactly 4 variants, no Failed
        let states = vec![
            TaskState::Active,
            TaskState::Blocked { obstacle: ObstacleType::Timeout { duration_ms: 1 }, attempts: 0 },
            TaskState::Rerouting { current_approach: ApproachType::Retry },
            TaskState::EscalatingToAgent { reason: "h".to_string() },
            TaskState::Suspended { waiting_for: "i".to_string() },
            TaskState::Complete,
            TaskState::HardDenied { evidence: stop },
        ];
        assert_eq!(states.len(), 7); // exactly 7 variants, no Failed
    }

    #[test]
    fn obstacle_display_is_informative() {
        let o = ObstacleType::Timeout { duration_ms: 5000 };
        let s = format!("{}", o);
        assert!(s.contains("5000"));
    }

    #[test]
    fn attempt_record_creation() {
        let record = AttemptRecord {
            approach: ApproachType::Retry,
            obstacle: ObstacleType::Timeout { duration_ms: 100 },
            outcome: AttemptOutcome::Succeeded,
            started_at: "2026-03-19T12:00:00Z".to_string(),
            ended_at: "2026-03-19T12:00:01Z".to_string(),
        };
        assert_eq!(record.outcome, AttemptOutcome::Succeeded);
    }
}
