//! Error types for the soul orientation layer.

use thiserror::Error;

/// Errors that can occur in the soul layer.
#[derive(Debug, Error)]
pub enum SoulError {
    /// The meaning graph has reached its maximum node capacity.
    #[error("meaning graph at capacity ({0} nodes)")]
    GraphAtCapacity(usize),

    /// The deepening was not confirmed by the principal.
    #[error("deepening not confirmed")]
    DeepeningNotConfirmed,

    /// The required reflection period has not yet elapsed.
    #[error("reflection period not elapsed (need {need_days} days, only {have_days} elapsed)")]
    ReflectionPeriodNotElapsed {
        /// Days required.
        need_days: i64,
        /// Days elapsed so far.
        have_days: i64,
    },

    /// Not enough data to perform the requested operation.
    #[error("insufficient data: {0}")]
    InsufficientData(String),

    /// The referenced deepening record was not found.
    #[error("deepening not found: {0}")]
    DeepeningNotFound(String),

    /// Write attempted through an unauthorized path.
    #[error("unauthorized write: {0}")]
    UnauthorizedWrite(String),
}
