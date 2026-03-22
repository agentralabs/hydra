//! Error types for hydra-skills.

use thiserror::Error;

/// Errors that can occur during skill operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SkillError {
    /// Skill execution was blocked by the constitution.
    #[error("Skill '{name}' blocked by constitutional gate: {reason}")]
    ConstitutionallyBlocked {
        /// The skill name that was blocked.
        name: String,
        /// The reason for the block.
        reason: String,
    },

    /// Skill not found in the registry.
    #[error("Skill '{name}' not found")]
    NotFound {
        /// The skill name that was not found.
        name: String,
    },

    /// The skill registry is at capacity.
    #[error("Skill registry at capacity ({max})")]
    RegistryFull {
        /// The maximum number of loaded skills.
        max: usize,
    },

    /// Skill is already loaded.
    #[error("Skill '{name}' already loaded")]
    AlreadyLoaded {
        /// The skill name that is already loaded.
        name: String,
    },

    /// Version conflict with an existing loaded skill.
    #[error("Skill '{name}' version conflict: loaded={loaded} requested={requested}")]
    VersionConflict {
        /// The skill name.
        name: String,
        /// The currently loaded version.
        loaded: String,
        /// The requested version.
        requested: String,
    },
}
