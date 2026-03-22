//! All error types for hydra-swarm.

use thiserror::Error;

/// Errors that can occur in swarm operations.
#[derive(Debug, Error)]
pub enum SwarmError {
    /// Not enough agents for the requested operation.
    #[error("Insufficient agents: need {needed}, have {have}")]
    InsufficientAgents {
        /// Number of agents required.
        needed: usize,
        /// Number of agents available.
        have: usize,
    },

    /// Underlying fleet error.
    #[error("Fleet error: {0}")]
    Fleet(#[from] hydra_fleet::FleetError),
}
