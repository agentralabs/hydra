//! Error types for hydra-resurrection.

use thiserror::Error;

/// All errors that can occur during checkpoint and resurrection operations.
#[derive(Debug, Error)]
pub enum ResurrectionError {
    /// Failed to serialize state to JSON.
    #[error("serialization failed: {0}")]
    Serialization(String),

    /// Failed to deserialize state from JSON.
    #[error("deserialization failed: {0}")]
    Deserialization(String),

    /// Checkpoint integrity verification failed (hash mismatch).
    #[error("integrity check failed: expected {expected}, got {actual}")]
    IntegrityFailure {
        /// The expected SHA256 hash.
        expected: String,
        /// The actual SHA256 hash.
        actual: String,
    },

    /// Checkpoint file exceeds the maximum allowed size.
    #[error("checkpoint too large: {size} bytes (max {max})")]
    CheckpointTooLarge {
        /// Actual size in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: usize,
    },

    /// Filesystem I/O error during checkpoint read/write.
    #[error("I/O error: {0}")]
    Io(String),

    /// No checkpoints available for reconstruction.
    #[error("no checkpoints available for reconstruction")]
    NoCheckpoints,

    /// Warm restart exceeded the target time.
    #[error("warm restart took {elapsed_ms}ms, target was {target_ms}ms")]
    WarmRestartSlow {
        /// Actual elapsed time in milliseconds.
        elapsed_ms: u64,
        /// Target time in milliseconds.
        target_ms: u64,
    },
}
