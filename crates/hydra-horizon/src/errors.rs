//! Error types for hydra-horizon.

use thiserror::Error;

/// Errors that can occur during horizon operations.
#[derive(Debug, Error, Clone)]
pub enum HorizonError {
    /// Attempted to contract the horizon. Horizons only expand.
    #[error("Horizon contraction attempted — growth invariant violated")]
    ContractionAttempted,

    /// A value was outside the valid range [0.0, 1.0].
    #[error("Horizon value {value:.4} out of range [0.0, 1.0]")]
    OutOfRange {
        /// The invalid value.
        value: f64,
    },
}
