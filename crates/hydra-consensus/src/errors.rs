use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConsensusError {
    #[error("Beliefs agree — no consensus needed (both: {topic})")]
    AlreadyAgree { topic: String },

    #[error("Insufficient evidence for belief '{topic}' — cannot resolve")]
    InsufficientEvidence { topic: String },
}
