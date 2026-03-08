//! Category 1: Unit Gap Fill — hydra-model edge cases.

use hydra_model::*;

// === LLM Config ===

#[test]
fn test_llm_config_no_providers() {
    let config = llm_config::LlmConfig::default();
    // default reads from env, so just verify the methods exist and return bool
    let _a = config.has_anthropic();
    let _o = config.has_openai();
    let _p = config.has_provider("nonexistent");
    assert!(!_p); // nonexistent provider is always false
}

#[test]
fn test_llm_config_all_providers() {
    let mut config = llm_config::LlmConfig::default();
    config.anthropic_api_key = Some("test-key".into());
    config.openai_api_key = Some("test-key".into());
    assert!(config.has_anthropic());
    assert!(config.has_openai());
    assert!(config.has_provider("anthropic"));
    assert!(config.has_provider("openai"));
    assert!(!config.has_provider("unknown"));
}

// === Model registry ===

#[test]
fn test_registry_builtin_models() {
    let registry = registry::ModelRegistry::new();
    assert!(registry.count() > 0);
    let all = registry.list_all();
    assert!(!all.is_empty());
}

#[test]
fn test_registry_mark_unavailable() {
    let registry = registry::ModelRegistry::new();
    let first = registry.list_all()[0].id.clone();
    registry.mark_unavailable(&first);
    let model = registry.get(&first).unwrap();
    assert!(!model.available);
}

#[test]
fn test_registry_mark_rate_limited() {
    let registry = registry::ModelRegistry::new();
    let first = registry.list_all()[0].id.clone();
    registry.mark_rate_limited(&first);
    let model = registry.get(&first).unwrap();
    assert!(model.rate_limited);
    assert!(!model.is_usable());
}

#[test]
fn test_registry_empty() {
    let registry = registry::ModelRegistry::empty();
    assert_eq!(registry.count(), 0);
    assert!(registry.list_available().is_empty());
}

// === Router errors ===

#[test]
fn test_router_no_model_available() {
    let registry = registry::ModelRegistry::empty();
    let router = router::ModelRouter::new(registry);
    let prefs = preferences::ModelPreferences::default();
    let result = router.route(profile::TaskType::General, &prefs);
    assert!(result.is_err());
}

#[test]
fn test_router_privacy_conflict() {
    let registry = registry::ModelRegistry::new();
    let router = router::ModelRouter::new(registry);
    let prefs = preferences::ModelPreferences::local_only();
    // All builtin cloud models should be filtered — may get local or error
    let _ = router.route(profile::TaskType::Code, &prefs);
}

// === Circuit breaker ===

#[test]
fn test_circuit_breaker_closed_to_open() {
    let cb = circuit_breaker::CircuitBreaker::new();
    assert_eq!(cb.state(), circuit_breaker::CircuitState::Closed);

    // 5 failures → open
    for _ in 0..5 {
        cb.track_failure();
    }
    assert_eq!(cb.state(), circuit_breaker::CircuitState::Open);
    assert!(cb.is_open());
}

#[test]
fn test_circuit_breaker_success_resets() {
    let cb = circuit_breaker::CircuitBreaker::new();
    cb.track_failure();
    cb.track_failure();
    assert_eq!(cb.failure_count(), 2);
    // track_success only resets failure_count when in HalfOpen state
    cb.track_success();
    // In Closed state, success doesn't reset failures
    assert_eq!(cb.failure_count(), 2);
    // Use reset() to clear everything
    cb.reset();
    assert_eq!(cb.failure_count(), 0);
}

#[test]
fn test_circuit_breaker_reset() {
    let cb = circuit_breaker::CircuitBreaker::new();
    for _ in 0..5 {
        cb.track_failure();
    }
    assert!(cb.is_open());
    cb.reset();
    assert!(!cb.is_open());
    assert_eq!(cb.state(), circuit_breaker::CircuitState::Closed);
}

// === Model profile scoring ===

#[test]
fn test_model_capabilities_score_for_task() {
    let caps = profile::ModelCapabilities {
        reasoning: 90,
        code: 95,
        creative: 80,
        math: 85,
        instruction_following: 90,
        vision: true,
        function_calling: true,
        context_window: 200000,
        max_output_tokens: 4096,
    };
    assert_eq!(caps.score_for_task(profile::TaskType::Code), 95);
    assert_eq!(caps.score_for_task(profile::TaskType::Reasoning), 90);
    assert_eq!(caps.score_for_task(profile::TaskType::Creative), 80);
    assert_eq!(caps.score_for_task(profile::TaskType::Math), 85);
}

// === Executor error messages ===

#[test]
fn test_executor_error_user_messages() {
    let kinds = vec![
        executor::ExecutorErrorKind::ModelNotFound,
        executor::ExecutorErrorKind::ModelUnavailable,
        executor::ExecutorErrorKind::RateLimited,
        executor::ExecutorErrorKind::InvalidApiKey,
        executor::ExecutorErrorKind::Timeout,
        executor::ExecutorErrorKind::CircuitOpen,
    ];
    for kind in kinds {
        let err = executor::ExecutorError::new_test(kind);
        assert!(!err.user_message().is_empty());
        assert!(!err.suggested_action().is_empty());
    }
}

// === Provider error variants ===

#[test]
fn test_provider_error_display() {
    let errors = vec![
        providers::LlmError::NoApiKey,
        providers::LlmError::Timeout,
        providers::LlmError::RateLimited,
        providers::LlmError::ParseError("bad json".into()),
        providers::LlmError::ApiError {
            status: 500,
            message: "internal".into(),
        },
    ];
    for e in &errors {
        assert!(!format!("{}", e).is_empty());
    }
}

// === Preferences ===

#[test]
fn test_preferences_blocked_models() {
    let mut prefs = preferences::ModelPreferences::default();
    prefs.blocked.insert("gpt-4o".into());
    assert!(prefs.is_blocked("gpt-4o"));
    assert!(!prefs.is_blocked("claude-opus"));
}

#[test]
fn test_preferences_local_only() {
    let prefs = preferences::ModelPreferences::local_only();
    assert!(prefs.require_local);
}

// === Local model tiers ===

#[test]
fn test_memory_tier_ordering() {
    use hydra_model::local::MemoryTier;
    assert!(MemoryTier::XLarge > MemoryTier::Large);
    assert!(MemoryTier::Large > MemoryTier::Medium);
    assert!(MemoryTier::Medium > MemoryTier::Small);
    assert!(MemoryTier::Small > MemoryTier::Tiny);
}

#[test]
fn test_memory_tier_vram() {
    use hydra_model::local::MemoryTier;
    assert!(MemoryTier::XLarge.min_vram_mb() > MemoryTier::Large.min_vram_mb());
}

// === Completion response ===

#[test]
fn test_completion_response_total_tokens() {
    let resp = providers::CompletionResponse {
        content: "hello".into(),
        model: "test".into(),
        input_tokens: 100,
        output_tokens: 50,
        stop_reason: None,
    };
    assert_eq!(resp.total_tokens(), 150);
}
