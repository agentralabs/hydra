//! Error types for hydra-prediction.

use thiserror::Error;

/// Errors that can occur in prediction operations.
#[derive(Debug, Error, Clone)]
pub enum PredictionError {
    /// Shadow execution failed.
    #[error("Shadow execution failed: {reason}")]
    ShadowFailed {
        /// Why the shadow execution failed.
        reason: String,
    },

    /// Insufficient history to make predictions.
    #[error("Insufficient history: have {available} intents, need {required}")]
    InsufficientHistory {
        /// Number of intents available.
        available: usize,
        /// Number required.
        required: usize,
    },

    /// Divergence between predicted and actual outcome exceeded threshold.
    #[error("Divergence exceeded: {divergence:.3} > threshold {threshold:.3}")]
    DivergenceExceeded {
        /// The actual divergence value.
        divergence: f64,
        /// The threshold that was exceeded.
        threshold: f64,
    },
}
