//! Error types for hydra-plastic.

use thiserror::Error;

/// Errors that can occur in plasticity operations.
#[derive(Debug, Error, Clone)]
pub enum PlasticError {
    /// The tensor has reached its maximum capacity.
    #[error("Plasticity tensor full (max {max} environments)")]
    TensorFull {
        /// The maximum number of environments allowed.
        max: usize,
    },

    /// A referenced environment was not found.
    #[error("Environment not found: '{name}'")]
    EnvironmentNotFound {
        /// The name of the missing environment.
        name: String,
    },
}
