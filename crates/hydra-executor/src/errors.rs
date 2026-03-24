//! Error types for the executor crate.

use thiserror::Error;

/// Errors that can occur during action execution.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ExecutorError {
    #[error("Action '{id}' not found in registry")]
    ActionNotFound { id: String },

    #[error("Receipt creation failed: {reason}")]
    ReceiptFailed { reason: String },

    #[error("Constitutional violation prevented execution: {law}")]
    ConstitutionalViolation { law: String },

    #[error("HardDenied: {evidence}")]
    HardDenied { evidence: String },

    #[error("All {attempts} approach attempts exhausted without HardDenied evidence")]
    ApproachesExhausted { attempts: u32 },

    #[error("Shadow execution failed: {reason}")]
    ShadowFailed { reason: String },

    #[error("Bridge '{name}' is not running")]
    BridgeNotRunning { name: String },

    #[error("Bridge '{name}' failed to start: {reason}")]
    BridgeStartFailed { name: String, reason: String },

    #[error("Bridge '{name}' send failed: {reason}")]
    BridgeSendFailed { name: String, reason: String },

    #[error("Local connector '{name}' operation '{operation}' failed: {reason}")]
    LocalOperationFailed {
        name: String,
        operation: String,
        reason: String,
    },

    #[error("Path traversal blocked: {path}")]
    PathTraversalBlocked { path: String },
}
