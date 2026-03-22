//! Error types for the transform crate.

use thiserror::Error;

/// Errors that can occur during data transformation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TransformError {
    #[error("No conversion path from '{from}' to '{to}'")]
    NoPath { from: String, to: String },

    #[error("Conversion chain too deep ({depth} > {max})")]
    ChainTooDeep { depth: usize, max: usize },

    #[error("Format '{name}' not registered")]
    FormatNotRegistered { name: String },

    #[error("Parse error for format '{format}': {reason}")]
    ParseError { format: String, reason: String },
}
