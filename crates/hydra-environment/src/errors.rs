//! Error types for environment detection and checking.

use thiserror::Error;

/// Errors that can occur during environment operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EnvironmentError {
    /// A required binary was not found on the system.
    #[error("Required binary '{binary}' not found — install hint: {hint}")]
    BinaryMissing {
        /// The binary that was not found.
        binary: String,
        /// A hint for how to install the binary.
        hint: String,
    },

    /// The environment does not have enough RAM.
    #[error("Insufficient RAM: {available_mb}MB available, {required_mb}MB required")]
    InsufficientRam {
        /// Available RAM in megabytes.
        available_mb: u64,
        /// Required RAM in megabytes.
        required_mb: u64,
    },

    /// The environment does not have enough disk space.
    #[error("Insufficient disk: {available_mb}MB available, {required_mb}MB required")]
    InsufficientDisk {
        /// Available disk in megabytes.
        available_mb: u64,
        /// Required disk in megabytes.
        required_mb: u64,
    },

    /// Requirements for a skill have not been registered.
    #[error("Skill '{skill}' environment requirements not registered")]
    RequirementsNotRegistered {
        /// The skill whose requirements are missing.
        skill: String,
    },

    /// An environment probe failed.
    #[error("Environment probe failed: {reason}")]
    ProbeFailed {
        /// The reason the probe failed.
        reason: String,
    },
}
