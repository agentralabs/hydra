//! Error types for the oracle crate.

use thiserror::Error;

/// Errors from oracle projections.
#[derive(Debug, Error)]
pub enum OracleError {
    /// No primitives provided for projection.
    #[error("insufficient context: no primitives provided")]
    InsufficientContext,

    /// Scenario limit exceeded.
    #[error("scenario limit reached: max {0}")]
    ScenarioLimitReached(usize),

    /// Invalid probability value.
    #[error("invalid probability {0}: must be in [0.0, 1.0]")]
    InvalidProbability(f64),
}
