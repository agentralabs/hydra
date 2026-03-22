//! Companion error types.

use thiserror::Error;

/// All errors that can occur within the hydra-companion.
#[derive(Debug, Error)]
pub enum CompanionError {
    /// Signal buffer is full and cannot accept more signals.
    #[error("Signal buffer full (capacity: {capacity})")]
    SignalBufferFull {
        /// The buffer capacity.
        capacity: usize,
    },

    /// Task limit reached — cannot start a new task.
    #[error("Task limit reached (max: {max})")]
    TaskLimitReached {
        /// The maximum number of concurrent tasks.
        max: usize,
    },

    /// A task was not found by ID.
    #[error("Task not found: {task_id}")]
    TaskNotFound {
        /// The task ID that was not found.
        task_id: String,
    },

    /// A task execution error.
    #[error("Task execution error for '{task_id}': {reason}")]
    TaskExecutionError {
        /// Which task failed.
        task_id: String,
        /// What went wrong.
        reason: String,
    },

    /// Signal classification error.
    #[error("Signal classification error: {reason}")]
    ClassificationError {
        /// What went wrong.
        reason: String,
    },
}
