use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider {provider} request failed: {reason}")]
    RequestFailed { provider: String, reason: String },

    #[error("no API key configured for provider {0}")]
    NoApiKey(String),

    #[error("response parse error from {provider}: {reason}")]
    ParseError { provider: String, reason: String },

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}
