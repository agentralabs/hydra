//! Error types for the comprehension engine.

use thiserror::Error;

/// Errors that can occur during comprehension.
#[derive(Debug, Error)]
pub enum ComprehensionError {
    /// The input string was empty or contained only whitespace.
    #[error("input is empty or whitespace-only")]
    EmptyInput,

    /// The input did not meet the minimum token count.
    #[error("input has {actual} tokens, minimum is {minimum}")]
    BelowMinimumLength {
        /// Actual number of tokens found.
        actual: usize,
        /// Required minimum.
        minimum: usize,
    },

    /// No axiom primitives could be extracted from the input.
    #[error("no axiom primitives could be mapped from input")]
    PrimitiveMappingFailed,

    /// Overall confidence fell below the acceptance threshold.
    #[error("confidence {confidence:.2} is below threshold {threshold:.2}")]
    LowConfidence {
        /// Computed confidence score.
        confidence: f64,
        /// Required threshold.
        threshold: f64,
    },
}
