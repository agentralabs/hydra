//! Error types for hydra-reflexive.

use thiserror::Error;

/// Errors that can occur during self-model operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ReflexiveError {
    /// A proposed modification was blocked by constitutional check.
    #[error("Modification blocked: {reason}")]
    ModificationBlocked {
        /// Why the modification was blocked.
        reason: String,
    },

    /// The requested rollback snapshot was not found.
    #[error("Rollback snapshot not found: {snapshot_id}")]
    RollbackNotFound {
        /// The ID of the missing snapshot.
        snapshot_id: String,
    },

    /// A capability with the given name was not found.
    #[error("Capability not found: {name}")]
    CapabilityNotFound {
        /// The capability name that was not found.
        name: String,
    },

    /// The self-model has reached its maximum capacity.
    #[error("Self-model full: {count}/{max} capabilities")]
    SelfModelFull {
        /// Current number of capabilities.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// A modification would violate the growth invariant.
    #[error("Growth invariant violated: capability count would decrease from {before} to {after}")]
    GrowthInvariantViolated {
        /// Count before modification.
        before: usize,
        /// Count after modification (less than before).
        after: usize,
    },
}
