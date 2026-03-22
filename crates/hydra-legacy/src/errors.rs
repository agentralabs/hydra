use thiserror::Error;

/// Errors that can occur during legacy artifact creation and management.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LegacyError {
    /// The source data does not have enough accumulated history.
    #[error("Insufficient operational history: {days} days (need {min})")]
    InsufficientHistory { days: u32, min: u32 },

    /// The legacy archive has reached its maximum capacity.
    #[error("Legacy archive at capacity ({max})")]
    ArchiveFull { max: usize },

    /// An artifact failed its integrity check.
    #[error("Artifact '{id}' integrity check failed")]
    IntegrityFailure { id: String },
}
