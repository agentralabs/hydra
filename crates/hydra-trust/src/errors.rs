//! All error types for hydra-trust.

use thiserror::Error;

/// Errors that can occur in trust operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TrustError {
    /// Trust score is out of the valid [0, 1] range.
    #[error("Trust score out of range: {value} (valid range: 0.0–1.0)")]
    ScoreOutOfRange {
        /// The invalid score value.
        value: f64,
    },

    /// Agent not found in the trust field.
    #[error("Agent not found: {agent_id}")]
    AgentNotFound {
        /// The missing agent's identifier.
        agent_id: String,
    },

    /// Trust field has reached maximum capacity.
    #[error("Trust field at capacity: {current}/{max} agents")]
    FieldAtCapacity {
        /// Current number of agents.
        current: usize,
        /// Maximum allowed agents.
        max: usize,
    },

    /// Operation not permitted on a quarantined agent.
    #[error("Agent '{agent_id}' is quarantined and cannot perform this operation")]
    AgentQuarantined {
        /// The quarantined agent's identifier.
        agent_id: String,
    },

    /// Constitutional violation detected.
    #[error("Constitutional violation by agent '{agent_id}': {reason}")]
    ConstitutionalViolation {
        /// The violating agent's identifier.
        agent_id: String,
        /// Description of the violation.
        reason: String,
    },

    /// Invalid tier for the requested operation.
    #[error("Invalid tier: {reason}")]
    InvalidTier {
        /// Description of the tier error.
        reason: String,
    },
}
