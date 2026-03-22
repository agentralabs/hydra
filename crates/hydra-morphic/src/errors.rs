//! Error types for hydra-morphic.

use thiserror::Error;

/// Errors that can occur during morphic identity operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum MorphicError {
    /// The morphic event history has reached its maximum capacity.
    #[error("Morphic history full: {count}/{max} events")]
    HistoryFull {
        /// Current number of events.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// Constitutional law blocked this identity operation.
    #[error("Constitutional violation: {reason}")]
    ConstitutionalViolation {
        /// Why the operation was blocked.
        reason: String,
    },
}
