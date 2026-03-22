use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum WisdomError {
    #[error("Insufficient intelligence for wisdom synthesis — need at least one Layer 4 input")]
    InsufficientIntelligence,

    #[error("Wisdom memory at capacity ({max})")]
    MemoryFull { max: usize },
}
