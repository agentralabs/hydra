//! Audit error types.

use thiserror::Error;

/// Errors that can occur during audit operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AuditError {
    /// The requested task was not found in the audit store.
    #[error("Task '{task_id}' not found in audit records")]
    TaskNotFound { task_id: String },

    /// The audit record store has reached its capacity limit.
    #[error("Audit record store at capacity ({max})")]
    StoreFull { max: usize },

    /// Trace has no events — nothing to narrate.
    #[error("Trace has no events — nothing to narrate")]
    EmptyTrace,

    /// Audit record is immutable — cannot be modified.
    #[error("Audit record '{id}' is immutable — cannot be modified")]
    ImmutableRecord { id: String },

    /// Constitutional law blocked this audit operation.
    #[error("Constitutional violation: {reason}")]
    ConstitutionalViolation { reason: String },
}
