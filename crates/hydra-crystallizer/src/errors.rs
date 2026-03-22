use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CrystallizerError {
    #[error("Insufficient data: need {min} records, have {count}")]
    InsufficientData { min: usize, count: usize },

    #[error("Artifact store at capacity ({max})")]
    StoreFull { max: usize },

    #[error("No patterns found for domain '{domain}'")]
    NoPatterns { domain: String },
}
