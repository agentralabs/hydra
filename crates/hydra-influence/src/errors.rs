use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum InfluenceError {
    #[error("Insufficient evidence: {count} (need {min})")]
    InsufficientEvidence { count: usize, min: usize },

    #[error("Confidence {confidence:.2} below minimum {min:.2} for publication")]
    LowConfidence { confidence: f64, min: f64 },

    #[error("Pattern '{id}' not found in registry")]
    PatternNotFound { id: String },

    #[error("Pattern '{id}' already adopted by lineage '{lineage}'")]
    AlreadyAdopted { id: String, lineage: String },

    #[error("Influence registry at capacity ({max})")]
    RegistryFull { max: usize },
}
