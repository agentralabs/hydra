//! Reasoning error types.

use thiserror::Error;

/// Errors that can occur during reasoning.
#[derive(Debug, Error)]
pub enum ReasoningError {
    /// The attention frame contains no focus or summary items.
    #[error("empty attention frame: no items to reason about")]
    EmptyAttentionFrame,

    /// All five reasoning modes produced zero conclusions.
    #[error("no conclusions: all reasoning modes returned empty")]
    NoConclusions,

    /// Synthesis confidence is below the configured threshold.
    #[error("low synthesis confidence: {confidence:.2} < threshold {threshold:.2}")]
    LowSynthesisConfidence {
        /// The computed synthesis confidence.
        confidence: f64,
        /// The threshold that was not met.
        threshold: f64,
    },
}
