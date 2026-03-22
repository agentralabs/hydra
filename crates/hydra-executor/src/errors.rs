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
}
