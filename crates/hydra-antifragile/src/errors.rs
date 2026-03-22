//! Error types for hydra-antifragile.

use thiserror::Error;

/// Errors that can occur in antifragile operations.
#[derive(Debug, Error, Clone)]
pub enum AntifragileError {
    /// The store has reached its maximum capacity.
    #[error("Antifragile store full (max {max} records)")]
    StoreFull {
        /// The maximum number of records allowed.
        max: usize,
    },
}
