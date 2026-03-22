use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PortfolioError {
    #[error("No objectives to allocate — add objectives first")]
    NoObjectives,

    #[error("Objective '{id}' not found")]
    ObjectiveNotFound { id: String },

    #[error("Portfolio at capacity ({max} objectives)")]
    PortfolioFull { max: usize },
}
