//! All error types for hydra-fleet.

use thiserror::Error;

/// Errors that can occur in fleet operations.
#[derive(Debug, Error)]
pub enum FleetError {
    /// Fleet has reached maximum agent capacity.
    #[error("Fleet at capacity: {current}/{max} agents")]
    FleetAtCapacity {
        /// Current number of agents in the fleet.
        current: usize,
        /// Maximum allowed agents.
        max: usize,
    },

    /// Constitutional check failed during spawn.
    #[error("Constitutional spawn rejection: {reason}")]
    ConstitutionalRejection {
        /// Reason the constitution rejected the spawn.
        reason: String,
    },

    /// Trust score too low for the requested operation.
    #[error("Trust too low: score {score:.4}, minimum {minimum:.4}")]
    TrustTooLow {
        /// The agent's current trust score.
        score: f64,
        /// The minimum required score.
        minimum: f64,
    },

    /// Agent not found in the fleet registry.
    #[error("Agent not found: {agent_id}")]
    AgentNotFound {
        /// The missing agent's identifier.
        agent_id: String,
    },

    /// Agent is quarantined and cannot accept tasks.
    #[error("Agent '{agent_id}' is quarantined")]
    AgentQuarantined {
        /// The quarantined agent's identifier.
        agent_id: String,
    },

    /// Agent's task queue is full.
    #[error("Task queue full for agent '{agent_id}': {current}/{max}")]
    TaskQueueFull {
        /// The agent whose queue is full.
        agent_id: String,
        /// Current queue size.
        current: usize,
        /// Maximum queue size.
        max: usize,
    },

    /// Result content exceeds the maximum allowed size.
    #[error("Result too large: {size} bytes, max {max} bytes")]
    ResultTooLarge {
        /// Actual size of the result.
        size: usize,
        /// Maximum permitted size.
        max: usize,
    },

    /// Underlying trust system error.
    #[error("Trust error: {0}")]
    Trust(#[from] hydra_trust::TrustError),
}
