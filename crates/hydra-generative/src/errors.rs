//! Error types for hydra-generative.

use thiserror::Error;

/// Errors that can occur in generative operations.
#[derive(Debug, Error, Clone)]
pub enum GenerativeError {
    /// Task description is empty.
    #[error("Task description is empty")]
    EmptyDescription,

    /// Decomposition exceeded the maximum number of primitives.
    #[error("Decomposition exceeded max primitives ({max})")]
    DecompositionTooLarge {
        /// The maximum allowed.
        max: usize,
    },

    /// Failed to add synthesized capability to genome.
    #[error("Failed to add to genome: {reason}")]
    GenomeAddFailed {
        /// The reason for the failure.
        reason: String,
    },
}
