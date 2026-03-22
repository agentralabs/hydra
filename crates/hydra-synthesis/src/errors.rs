//! Synthesis error types.

use thiserror::Error;

/// Errors that can occur during synthesis operations.
#[derive(Debug, Error)]
pub enum SynthesisError {
    /// No structural patterns exist in the library.
    #[error("no structural patterns available for synthesis")]
    NoStructuralPatterns,

    /// The pattern library has reached its capacity limit.
    #[error("pattern library at capacity ({max})")]
    LibraryAtCapacity {
        /// The maximum library size.
        max: usize,
    },

    /// Not enough axiom primitives to form a pattern.
    #[error("insufficient primitives: have {have}, need at least 1")]
    InsufficientPrimitives {
        /// Number of primitives found.
        have: usize,
    },
}
