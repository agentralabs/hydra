use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum OmniscienceError {
    #[error("Knowledge gap '{topic}' could not be resolved from any source")]
    GapUnresolvable { topic: String },

    #[error("All acquisition sources exhausted for '{topic}'")]
    SourcesExhausted { topic: String },

    #[error("Acquisition result below confidence threshold: {confidence:.2} < {threshold:.2}")]
    LowConfidence { confidence: f64, threshold: f64 },

    #[error("Gap registry at capacity ({max})")]
    RegistryFull { max: usize },
}

impl OmniscienceError {
    /// True if this gap should be escalated to the principal.
    pub fn requires_human(&self) -> bool {
        matches!(self,
            Self::GapUnresolvable { .. } | Self::SourcesExhausted { .. })
    }
}
