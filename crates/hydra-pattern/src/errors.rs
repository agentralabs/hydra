use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PatternError {
    #[error("Pattern library at capacity ({max})")]
    LibraryFull { max: usize },

    #[error("Pattern '{id}' not found")]
    PatternNotFound { id: String },
}
