//! Learning error types.

use thiserror::Error;

/// Errors that can occur during learning operations.
#[derive(Debug, Error)]
pub enum LearningError {
    /// Not enough observations to propose an adjustment.
    #[error("insufficient observations: have {have}, need {need}")]
    InsufficientObservations {
        /// Number of observations currently recorded.
        have: usize,
        /// Minimum required.
        need: usize,
    },

    /// The specified domain is not valid or not tracked.
    #[error("invalid domain: {domain}")]
    InvalidDomain {
        /// The domain label that was not found.
        domain: String,
    },
}
