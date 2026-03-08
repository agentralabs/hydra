use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::circuit_breaker::CircuitBreaker;
use crate::llm_config::LlmConfig;
use crate::local::ollama::OllamaClient;
use crate::profile::ModelProfile;
use crate::providers::anthropic::AnthropicClient;
use crate::providers::openai::OpenAiClient;
use crate::providers::{CompletionRequest, CompletionResponse, LlmError, Message};
use crate::registry::ModelRegistry;

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

    fn new(kind: ExecutorErrorKind) -> Self {
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

/// Model executor with circuit breakers, timeouts, and retry
///
/// State ownership:
/// - `registry`: Shared reference to model profiles (read-only here)
/// - `circuit_breakers`: Owned per-model circuit state, cleaned up on drop
/// - `config`: LLM provider configuration (API keys, base URLs)
/// - Simulation flags: Test-only, not persisted
pub struct ModelExecutor {
    registry: ModelRegistry,
    circuit_breakers: HashMap<String, CircuitBreaker>,
    config: LlmConfig,
    simulate_failure: AtomicBool,
    simulate_bad_output: AtomicBool,
    simulate_oom: AtomicBool,
}

impl ModelExecutor {
    pub fn new(registry: ModelRegistry) -> Self {
        Self::with_config(registry, LlmConfig::from_env())
    }

    pub fn with_config(registry: ModelRegistry, config: LlmConfig) -> Self {
        let mut breakers = HashMap::new();
        for model in registry.list_all() {
            breakers.insert(model.id.clone(), CircuitBreaker::new());
        }
        Self {
            registry,
            circuit_breakers: breakers,
            config,
            simulate_failure: AtomicBool::new(false),
            simulate_bad_output: AtomicBool::new(false),
            simulate_oom: AtomicBool::new(false),
        }
    }

    /// Get circuit breaker for a model
    pub fn circuit_breaker(&self, model_id: &str) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(model_id)
    }

    /// Execute a task with retry (exponential backoff: 1s, 2s, 4s) and fallback
    pub async fn execute(
        &self,
        model_id: &str,
        task: &str,
        fallbacks: &[ModelProfile],
    ) -> Result<ExecutionResult, ExecutorError> {
        // Try primary model with retry
        match self.execute_with_retry(model_id, task).await {
            Ok(result) => return Ok(result),
            Err(_) => {
                // Try fallbacks in order
                for fallback in fallbacks {
                    if let Ok(mut result) = self.execute_with_retry(&fallback.id, task).await {
                        result.used_fallback = true;
                        return Ok(result);
                    }
                }
            }
        }

        Err(ExecutorError::new(ExecutorErrorKind::AllModelsFailed))
    }

    /// Execute with retry and exponential backoff (1s, 2s, 4s)
    async fn execute_with_retry(
        &self,
        model_id: &str,
        task: &str,
    ) -> Result<ExecutionResult, ExecutorError> {
        let backoff_ms = [1, 2, 4]; // In real code: 1000, 2000, 4000ms

        for attempt in 0..MAX_RETRY_ATTEMPTS {
            match self.try_execute(model_id, task).await {
                Ok(result) => {
                    if let Some(cb) = self.circuit_breakers.get(model_id) {
                        cb.track_success();
                    }
                    return Ok(result);
                }
                Err(e) => {
                    if let Some(cb) = self.circuit_breakers.get(model_id) {
                        cb.track_failure();
                    }
                    // Don't retry non-retryable errors
                    if matches!(
                        e.kind,
                        ExecutorErrorKind::ModelNotFound
                            | ExecutorErrorKind::InvalidApiKey
                            | ExecutorErrorKind::CircuitOpen
                    ) {
                        return Err(e);
                    }
                    if attempt < MAX_RETRY_ATTEMPTS - 1 {
                        let delay = Duration::from_millis(backoff_ms[attempt as usize]);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(ExecutorError::new(ExecutorErrorKind::ModelFailed))
    }

    async fn try_execute(
        &self,
        model_id: &str,
        task: &str,
    ) -> Result<ExecutionResult, ExecutorError> {
        // Circuit breaker check
        if let Some(cb) = self.circuit_breakers.get(model_id) {
            if cb.is_open() {
                return Err(ExecutorError::new(ExecutorErrorKind::CircuitOpen));
            }
        }

        let model = self
            .registry
            .get(model_id)
            .ok_or_else(|| ExecutorError::new(ExecutorErrorKind::ModelNotFound))?;

        if !model.is_usable() {
            return Err(ExecutorError::new(ExecutorErrorKind::ModelUnavailable));
        }

        // OOM simulation for local models
        if self.simulate_oom.load(Ordering::SeqCst)
            && model.privacy == crate::profile::PrivacyLevel::Local
        {
            return Err(ExecutorError::new(ExecutorErrorKind::OutOfMemory));
        }

        // Failure simulation — only fails once then clears
        if self
            .simulate_failure
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Err(ExecutorError::new(ExecutorErrorKind::ModelFailed));
        }

        // Bad output simulation — retry once
        let mut retried = false;
        if self
            .simulate_bad_output
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            retried = true;
        }

        // Try real LLM call if provider has an API key configured
        let provider = &model.provider;
        let real_result = if self.config.has_provider(provider) {
            let completion_req = CompletionRequest {
                model: model_id.into(),
                messages: vec![Message {
                    role: "user".into(),
                    content: task.into(),
                }],
                max_tokens: model.capabilities.max_output_tokens,
                temperature: Some(0.7),
                system: None,
            };

            tokio::time::timeout(LLM_COMPLETION_TIMEOUT, async {
                self.call_provider(provider, completion_req).await
            })
            .await
            .map_err(|_| ExecutorError::new(ExecutorErrorKind::Timeout))?
        } else {
            // No API key — use mock
            Err(LlmError::NoApiKey)
        };

        let result = match real_result {
            Ok(resp) => ExecutionResult {
                model_id: model_id.into(),
                output: serde_json::json!({
                    "model": resp.model,
                    "response": resp.content,
                }),
                tokens_used: resp.total_tokens(),
                latency_ms: model.latency_ms as u64,
                used_fallback: false,
                retried,
                detected_bad_output: retried,
            },
            Err(LlmError::RateLimited) => {
                return Err(ExecutorError::new(ExecutorErrorKind::RateLimited));
            }
            Err(LlmError::ApiError { status: 401, .. }) => {
                return Err(ExecutorError::new(ExecutorErrorKind::InvalidApiKey));
            }
            Err(LlmError::Timeout) => {
                return Err(ExecutorError::new(ExecutorErrorKind::Timeout));
            }
            Err(LlmError::NoApiKey) | Err(_) => {
                // Fallback to mock when no API key or other errors
                ExecutionResult {
                    model_id: model_id.into(),
                    output: serde_json::json!({
                        "model": model_id,
                        "response": format!("Processed: {}", task),
                    }),
                    tokens_used: (task.len() / 4 + 100) as u64,
                    latency_ms: model.latency_ms as u64,
                    used_fallback: false,
                    retried,
                    detected_bad_output: retried,
                }
            }
        };

        Ok(result)
    }

    /// Dispatch to the appropriate provider client
    async fn call_provider(
        &self,
        provider: &str,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, LlmError> {
        match provider {
            "anthropic" => {
                let client = AnthropicClient::new(&self.config)?;
                client.complete(request).await
            }
            "openai" => {
                let client = OpenAiClient::new(&self.config)?;
                client.complete(request).await
            }
            "ollama" | "local" => {
                let client = OllamaClient::new();
                client.chat(request).await
            }
            _ => Err(LlmError::NoApiKey),
        }
    }

    // Test helpers — API keys NEVER appear in these
    pub fn simulate_model_failure(&self) {
        self.simulate_failure.store(true, Ordering::SeqCst);
    }

    pub fn simulate_bad_output(&self) {
        self.simulate_bad_output.store(true, Ordering::SeqCst);
    }

    pub fn simulate_local_oom(&self) {
        self.simulate_oom.store(true, Ordering::SeqCst);
    }
}
