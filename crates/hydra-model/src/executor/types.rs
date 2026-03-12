use std::time::Duration;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════
// TIMEOUT CONSTANTS (from arch spec §2.4)
// ═══════════════════════════════════════════════════════════

/// LLM simple completion timeout
pub const LLM_COMPLETION_TIMEOUT: Duration = Duration::from_secs(30);
/// LLM streaming first token timeout
pub const LLM_FIRST_TOKEN_TIMEOUT: Duration = Duration::from_secs(10);
/// LLM streaming total timeout
pub const LLM_STREAMING_TIMEOUT: Duration = Duration::from_secs(60);
/// Model health check timeout
pub const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);
/// Max retry attempts
pub const MAX_RETRY_ATTEMPTS: u32 = 3;

// ═══════════════════════════════════════════════════════════
// ERROR CLASSIFICATION (from arch spec §4.1)
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
    Fatal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    UserError,
    DependencyError,
    ResourceError,
    InternalError,
    SecurityError,
}

/// Result of model execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub model_id: String,
    pub output: serde_json::Value,
    pub tokens_used: u64,
    pub latency_ms: u64,
    pub used_fallback: bool,
    pub retried: bool,
    pub detected_bad_output: bool,
}

/// Classified execution error — has severity, category, user message, suggested action
#[derive(Debug, Clone)]
pub struct ExecutorError {
    pub kind: ExecutorErrorKind,
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorErrorKind {
    ModelNotFound,
    ModelUnavailable,
    ModelFailed,
    AllModelsFailed,
    OutOfMemory,
    RateLimited,
    InvalidApiKey,
    Timeout,
    CircuitOpen,
}

impl ExecutorError {
    /// Create a classified error
    pub fn new_test(kind: ExecutorErrorKind) -> Self {
        Self::new(kind)
    }

    pub(crate) fn new(kind: ExecutorErrorKind) -> Self {
        let (severity, category) = match &kind {
            ExecutorErrorKind::ModelNotFound => (ErrorSeverity::Error, ErrorCategory::UserError),
            ExecutorErrorKind::ModelUnavailable => {
                (ErrorSeverity::Warning, ErrorCategory::DependencyError)
            }
            ExecutorErrorKind::ModelFailed => {
                (ErrorSeverity::Error, ErrorCategory::DependencyError)
            }
            ExecutorErrorKind::AllModelsFailed => {
                (ErrorSeverity::Critical, ErrorCategory::DependencyError)
            }
            ExecutorErrorKind::OutOfMemory => {
                (ErrorSeverity::Critical, ErrorCategory::ResourceError)
            }
            ExecutorErrorKind::RateLimited => {
                (ErrorSeverity::Warning, ErrorCategory::DependencyError)
            }
            ExecutorErrorKind::InvalidApiKey => {
                (ErrorSeverity::Error, ErrorCategory::SecurityError)
            }
            ExecutorErrorKind::Timeout => (ErrorSeverity::Warning, ErrorCategory::DependencyError),
            ExecutorErrorKind::CircuitOpen => {
                (ErrorSeverity::Warning, ErrorCategory::DependencyError)
            }
        };
        Self {
            kind,
            severity,
            category,
        }
    }

    /// Human-friendly message — NEVER contains API keys
    pub fn user_message(&self) -> &'static str {
        match self.kind {
            ExecutorErrorKind::ModelNotFound => "Model not found in registry. Check the model ID and try again.",
            ExecutorErrorKind::ModelUnavailable => "Model is currently unavailable. It may be offline or rate-limited. Try a different model.",
            ExecutorErrorKind::ModelFailed => "Model execution failed. The service may be experiencing issues. Trying fallback models.",
            ExecutorErrorKind::AllModelsFailed => "All models failed for this task. Check your network connection and model configuration.",
            ExecutorErrorKind::OutOfMemory => "Local model ran out of memory. Use a smaller model or free up system resources.",
            ExecutorErrorKind::RateLimited => "Model is rate-limited. Wait a moment before retrying. Consider using a different model.",
            ExecutorErrorKind::InvalidApiKey => "API authentication failed. Check your credentials in the secure keychain. Never store keys in config files.",
            ExecutorErrorKind::Timeout => "Model request timed out after 30 seconds. The service may be overloaded. Try again or use a faster model.",
            ExecutorErrorKind::CircuitOpen => "This model's circuit breaker is open due to repeated failures. It will be retried automatically in 30 seconds.",
        }
    }

    /// Suggested action for recovery
    pub fn suggested_action(&self) -> &'static str {
        match self.kind {
            ExecutorErrorKind::ModelNotFound => "Check available models with 'hydra model list'.",
            ExecutorErrorKind::ModelUnavailable => "Try a different model or wait for recovery.",
            ExecutorErrorKind::ModelFailed => {
                "Check logs for details. The fallback chain will be tried."
            }
            ExecutorErrorKind::AllModelsFailed => {
                "Check network connectivity and API status pages."
            }
            ExecutorErrorKind::OutOfMemory => "Reduce context size or switch to a cloud model.",
            ExecutorErrorKind::RateLimited => "Wait 60 seconds or switch to a different provider.",
            ExecutorErrorKind::InvalidApiKey => "Re-enter your API key in the secure keychain.",
            ExecutorErrorKind::Timeout => "Increase timeout in config or use a faster model.",
            ExecutorErrorKind::CircuitOpen => {
                "Wait for automatic recovery or reset with 'hydra model reset'."
            }
        }
    }
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.user_message(), self.suggested_action())
    }
}

impl std::error::Error for ExecutorError {}
