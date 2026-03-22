//! Error types for hydra-reach.

use thiserror::Error;

/// Errors that can occur during reach operations.
#[derive(Debug, Error, Clone)]
pub enum ReachError {
    /// Device authentication failed.
    #[error("Device authentication failed for '{device_id}'")]
    AuthenticationFailed {
        /// The device that failed authentication.
        device_id: String,
    },

    /// Device not connected.
    #[error("Device '{device_id}' not connected")]
    DeviceNotConnected {
        /// The device ID that is not connected.
        device_id: String,
    },

    /// Maximum device connections reached.
    #[error("Maximum device connections reached ({max})")]
    MaxConnectionsReached {
        /// The maximum allowed connections.
        max: usize,
    },

    /// Session handoff failed.
    #[error("Session handoff failed: {reason}")]
    HandoffFailed {
        /// The reason for the handoff failure.
        reason: String,
    },

    /// Server bind failed.
    #[error("Server bind failed on port {port}: {reason}")]
    ServerBindFailed {
        /// The port that failed to bind.
        port: u16,
        /// The reason for the failure.
        reason: String,
    },
}
