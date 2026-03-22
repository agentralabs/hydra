use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CollectiveError {
    #[error("Insufficient observations for topic '{topic}': {count} < {min}")]
    InsufficientObservations {
        topic: String,
        count: usize,
        min: usize,
    },

    #[error("Aggregated confidence {confidence:.2} below minimum {min:.2}")]
    LowConfidence { confidence: f64, min: f64 },

    #[error("Observation store at capacity ({max})")]
    StoreFull { max: usize },
}
