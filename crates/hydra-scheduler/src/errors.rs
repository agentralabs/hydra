use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum SchedulerError {
    #[error("Job queue at capacity ({max})")]
    QueueFull { max: usize },

    #[error("Job '{id}' not found")]
    JobNotFound { id: String },

    #[error("Invalid schedule expression: {expression}")]
    InvalidSchedule { expression: String },

    #[error("Job '{id}' already exists")]
    DuplicateJob { id: String },
}
