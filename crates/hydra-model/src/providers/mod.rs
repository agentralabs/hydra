pub mod anthropic;
pub mod openai;

use serde::{Deserialize, Serialize};

/// Unified completion request sent to any provider
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
    pub system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Unified completion response from any provider
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub stop_reason: Option<String>,
}

impl CompletionResponse {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Provider error
#[derive(Debug)]
pub enum LlmError {
    NoApiKey,
    HttpError(reqwest::Error),
    ApiError { status: u16, message: String },
    ParseError(String),
    Timeout,
    RateLimited,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NoApiKey => write!(f, "API key not configured"),
            LlmError::HttpError(e) => write!(f, "HTTP error: {}", e),
            LlmError::ApiError { status, message } => {
                write!(f, "API error {}: {}", status, message)
            }
            LlmError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LlmError::Timeout => write!(f, "Request timed out"),
            LlmError::RateLimited => write!(f, "Rate limited"),
        }
    }
}

impl std::error::Error for LlmError {}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            LlmError::Timeout
        } else {
            LlmError::HttpError(e)
        }
    }
}
