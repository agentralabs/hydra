//! Web engine error types.

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("All search engines failed for '{query}'")]
    AllEnginesFailed { query: String },

    #[error("Deep fetch failed for {url}: {reason}")]
    DeepFetchFailed { url: String, reason: String },

    #[error("Cache error: {reason}")]
    CacheError { reason: String },

    #[error("Synthesis failed: {reason}")]
    SynthesisFailed { reason: String },

    #[error("Invalid request: {reason}")]
    RequestInvalid { reason: String },

    #[error("HTTP error: {0}")]
    Http(String),
}
