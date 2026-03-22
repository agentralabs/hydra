//! Error types for hydra-belief.

use thiserror::Error;

/// Errors that can occur in belief operations.
#[derive(Debug, Error, Clone)]
pub enum BeliefError {
    /// Belief not found by the given identifier.
    #[error("Belief not found: '{id}'")]
    NotFound {
        /// The ID that was not found.
        id: String,
    },

    /// Attempted to revise an immutable belief.
    #[error("Belief '{id}' is immutable and cannot be revised")]
    ImmutableBelief {
        /// The ID of the immutable belief.
        id: String,
    },

    /// Belief store has reached its maximum capacity.
    #[error("Belief store full (max {max} beliefs)")]
    BeliefSetFull {
        /// The maximum capacity.
        max: usize,
    },

    /// A revision cycle was detected (A revises B which revises A).
    #[error("Revision cycle detected involving beliefs: {ids:?}")]
    RevisionCycle {
        /// The belief IDs involved in the cycle.
        ids: Vec<String>,
    },

    /// An AGM postulate was violated during revision.
    #[error("AGM postulate violation: {postulate} — {reason}")]
    AgmPostulateViolation {
        /// Which postulate was violated.
        postulate: String,
        /// Details of the violation.
        reason: String,
    },

    /// Constitutional law blocked this revision.
    #[error("Revision denied: {reason}")]
    RevisionDenied {
        /// Why the revision was blocked.
        reason: String,
    },
}
