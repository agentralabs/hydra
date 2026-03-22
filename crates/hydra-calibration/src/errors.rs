use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum CalibrationError {
    #[error("Insufficient records for domain '{domain}' ({count} < {min})")]
    InsufficientRecords {
        domain: String,
        count: usize,
        min: usize,
    },

    #[error("Record '{id}' already has an outcome recorded")]
    OutcomeAlreadyRecorded { id: String },

    #[error("Calibration record store at capacity ({max})")]
    StoreFull { max: usize },
}
