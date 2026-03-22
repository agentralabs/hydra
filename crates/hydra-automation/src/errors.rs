use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AutomationError {
    #[error("Proposal '{id}' not found")]
    ProposalNotFound { id: String },

    #[error("Pattern '{pattern}' has insufficient observations ({count} < {threshold})")]
    InsufficientObservations {
        pattern: String,
        count: usize,
        threshold: usize,
    },

    #[error("Skill generation failed: {reason}")]
    GenerationFailed { reason: String },

    #[error("Proposal already exists for pattern '{pattern}'")]
    DuplicateProposal { pattern: String },
}
