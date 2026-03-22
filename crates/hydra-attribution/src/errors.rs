//! Attribution error types.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum AttributionError {
    #[error("Settlement record '{id}' has no attributable cost items")]
    NoCostItems { id: String },

    #[error("Attribution chain depth exceeded ({max})")]
    ChainTooDeep { max: usize },

    #[error("Attribution store at capacity ({max})")]
    StoreFull { max: usize },
}
