//! Error types for hydra-context.

use thiserror::Error;

/// Errors that can occur during context operations.
#[derive(Debug, Error)]
pub enum ContextError {
    /// The context window has expired (TTL exceeded).
    #[error("context window expired after TTL")]
    WindowExpired,

    /// No active context is available.
    #[error("no active context available")]
    NoActiveContext,

    /// Prediction data is unavailable.
    #[error("prediction data unavailable")]
    PredictionUnavailable,
}
