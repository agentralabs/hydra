use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum DiplomatError {
    #[error("Insufficient participants: {count} < {min}")]
    InsufficientParticipants { count: usize, min: usize },

    #[error("No agreement reached: {reason}")]
    NoAgreement { reason: String },

    #[error("Session '{id}' is not open for contributions")]
    SessionClosed { id: String },

    #[error("Participant '{peer_id}' already submitted a stance")]
    DuplicateStance { peer_id: String },
}
