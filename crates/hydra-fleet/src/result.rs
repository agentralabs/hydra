//! Agent result and receipt system.
//! Every result MUST be receipted before consumption.

use crate::constants::RESULT_MAX_CONTENT_BYTES;
use crate::errors::FleetError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The outcome category of an agent's work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultOutcome {
    /// Task completed successfully.
    Success,
    /// Task partially completed.
    PartialSuccess,
    /// Task failed.
    Failed,
    /// Task was blocked by an external dependency.
    Blocked,
    /// Task was stopped due to a constitutional violation.
    ConstitutionalViolation,
}

impl std::fmt::Display for ResultOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Success => "Success",
            Self::PartialSuccess => "PartialSuccess",
            Self::Failed => "Failed",
            Self::Blocked => "Blocked",
            Self::ConstitutionalViolation => "ConstitutionalViolation",
        };
        write!(f, "{label}")
    }
}

/// The result produced by a fleet agent after completing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Unique identifier for this result.
    pub id: Uuid,
    /// Which agent produced this result.
    pub agent_id: Uuid,
    /// Which task this result is for.
    pub task_id: Uuid,
    /// The outcome category.
    pub outcome: ResultOutcome,
    /// The content payload.
    pub content: String,
    /// When this result was produced.
    pub produced_at: DateTime<Utc>,
}

impl AgentResult {
    /// Create a new agent result, validating content size.
    pub fn new(
        agent_id: Uuid,
        task_id: Uuid,
        outcome: ResultOutcome,
        content: impl Into<String>,
    ) -> Result<Self, FleetError> {
        let content_str = content.into();
        if content_str.len() > RESULT_MAX_CONTENT_BYTES {
            return Err(FleetError::ResultTooLarge {
                size: content_str.len(),
                max: RESULT_MAX_CONTENT_BYTES,
            });
        }
        Ok(Self {
            id: Uuid::new_v4(),
            agent_id,
            task_id,
            outcome,
            content: content_str,
            produced_at: Utc::now(),
        })
    }
}

/// A receipt issued BEFORE a result is consumed.
/// Proves the result existed and was acknowledged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultReceipt {
    /// Unique receipt identifier.
    pub receipt_id: Uuid,
    /// The result this receipt covers.
    pub result_id: Uuid,
    /// The agent that produced the result.
    pub agent_id: Uuid,
    /// The outcome that was receipted.
    pub outcome: ResultOutcome,
    /// When the receipt was issued.
    pub issued_at: DateTime<Utc>,
}

impl ResultReceipt {
    /// Issue a receipt for an agent result.
    /// This MUST be called before consuming the result.
    pub fn issue(result: &AgentResult) -> Self {
        Self {
            receipt_id: Uuid::new_v4(),
            result_id: result.id,
            agent_id: result.agent_id,
            outcome: result.outcome.clone(),
            issued_at: Utc::now(),
        }
    }
}
