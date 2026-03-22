//! Error types for protocol operations.

use thiserror::Error;

/// Errors that can occur during protocol operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProtocolError {
    #[error("No protocol adapter found for target '{target}'")]
    NoAdapterFound { target: String },

    #[error("Protocol '{protocol}' not supported")]
    ProtocolNotSupported { protocol: String },

    #[error("Connection failed after {attempts} attempts: {reason}")]
    ConnectionFailed { attempts: u32, reason: String },

    #[error(
        "Protocol mismatch: target speaks '{target_protocol}', \
             adapter expects '{adapter_protocol}'"
    )]
    ProtocolMismatch {
        target_protocol: String,
        adapter_protocol: String,
    },

    #[error("Authentication denied: {reason}")]
    AuthDenied { reason: String },
}
