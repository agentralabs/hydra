//! Error types for hydra-cartography.

use thiserror::Error;

/// Errors that can occur in cartography operations.
#[derive(Debug, Error, Clone)]
pub enum CartographyError {
    /// The atlas has reached its maximum capacity.
    #[error("Atlas full (max {max} profiles)")]
    AtlasFull {
        /// The maximum number of profiles allowed.
        max: usize,
    },

    /// A referenced system profile was not found.
    #[error("System profile not found: '{name}'")]
    ProfileNotFound {
        /// The name of the missing profile.
        name: String,
    },

    /// A system profile with the same name already exists.
    #[error("System profile already exists: '{name}'")]
    ProfileAlreadyExists {
        /// The name of the duplicate profile.
        name: String,
    },
}
