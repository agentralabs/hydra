//! Error types for hydra-language.

use thiserror::Error;

/// Errors that can occur during language analysis.
#[derive(Debug, Error)]
pub enum LanguageError {
    /// The input text is empty.
    #[error("empty input — cannot analyze")]
    EmptyInput,

    /// Insufficient context for meaningful analysis.
    #[error("insufficient context for language analysis")]
    InsufficientContext,
}
