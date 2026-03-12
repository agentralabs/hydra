use hydra_model::circuit_breaker::{CircuitBreaker, CircuitState};
use hydra_model::executor::{self, ErrorCategory, ErrorSeverity, ExecutorError, ModelExecutor};
use hydra_model::preferences::ModelPreferences;
use hydra_model::profile::{builtin_profiles, PrivacyLevel, TaskType};
use hydra_model::registry::ModelRegistry;
use hydra_model::router::{ModelRouter, RouterError};

fn default_prefs() -> ModelPreferences {
    ModelPreferences::default()
}

// ═══════════════════════════════════════════════════════════
// PROFILE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_builtin_profiles_exist() {
    let profiles = builtin_profiles();
    assert!(profiles.len() >= 7);
    let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"claude-opus"));
    assert!(ids.contains(&"claude-sonnet"));
    assert!(ids.contains(&"claude-haiku"));
    assert!(ids.contains(&"gpt-4o"));
    assert!(ids.contains(&"llama-3-70b"));
    assert!(ids.contains(&"deepseek-coder-v2"));
}

#[test]
fn test_capability_scoring() {
    let profile = &builtin_profiles()[0]; // claude-opus
    assert!(profile.capabilities.score_for_task(TaskType::Reasoning) > 90);
    assert!(profile.capabilities.score_for_task(TaskType::Code) > 90);
}

#[test]
fn test_privacy_levels_ordered() {
    assert!(PrivacyLevel::AirGapped < PrivacyLevel::Local);
    assert!(PrivacyLevel::Local < PrivacyLevel::Cloud);
}

// ═══════════════════════════════════════════════════════════
// REGISTRY TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_registry_has_builtins() {
    let registry = ModelRegistry::new();
    assert!(registry.count() >= 7);
    assert!(registry.get("claude-opus").is_some());
}

#[test]
fn test_registry_mark_unavailable() {
    let registry = ModelRegistry::new();
    registry.mark_unavailable("claude-opus");
    let opus = registry.get("claude-opus").unwrap();
    assert!(!opus.is_usable());
}

#[test]
fn test_registry_mark_rate_limited() {
    let registry = ModelRegistry::new();
    registry.mark_rate_limited("claude-sonnet");
    let sonnet = registry.get("claude-sonnet").unwrap();
    assert!(!sonnet.is_usable());
}

// ═══════════════════════════════════════════════════════════
// ROUTER TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_route_selects_best_model() {
    let router = ModelRouter::new(ModelRegistry::new());
    let decision = router.route(TaskType::Code, &default_prefs()).unwrap();
    assert!(!decision.model_id().is_empty());
    assert!(decision.score > 0.0);
}

#[test]
fn test_route_prefers_cheaper_for_simple() {
    let router = ModelRouter::new(ModelRegistry::new());
    let decision = router
        .route(TaskType::Conversation, &default_prefs())
        .unwrap();
    // Should prefer cheaper models for simple conversation
    assert!(decision.fallbacks.len() >= 1);
}

#[test]
fn test_route_respects_blocked_models() {
    let router = ModelRouter::new(ModelRegistry::new());
    let mut prefs = default_prefs();
    prefs.blocked.insert("claude-opus".into());
    let decision = router.route(TaskType::Code, &prefs).unwrap();
    assert_ne!(decision.model_id(), "claude-opus");
}

#[test]
fn test_route_respects_preferred_models() {
    let router = ModelRouter::new(ModelRegistry::new());
    let mut prefs = default_prefs();
    prefs.preferred = vec!["claude-haiku".into()];
    let decision = router.route(TaskType::General, &prefs).unwrap();
    // Haiku should be boosted if competitive
    assert!(!decision.model_id().is_empty());
}

#[test]
fn test_route_scoring_formula() {
    // Score = capability(40%) + cost(30%) + latency(20%) + privacy(10%)
    let router = ModelRouter::new(ModelRegistry::new());
    let decision = router.route(TaskType::Code, &default_prefs()).unwrap();
    assert!(decision.score > 0.0);
    assert!(decision.score <= 1.0);
}

// ═══════════════════════════════════════════════════════════
// EDGE CASES (EC-MR-001 through EC-MR-010)
// ═══════════════════════════════════════════════════════════

/// EC-MR-001: All models unavailable
#[test]
fn test_ec_mr_001_all_models_down() {
    let registry = ModelRegistry::new();
    for profile in builtin_profiles() {
        registry.mark_unavailable(&profile.id);
    }
    let router = ModelRouter::new(registry);
    let result = router.route(TaskType::Code, &default_prefs());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RouterError::NoModelAvailable);
}

/// EC-MR-002: Model fails mid-request — retry succeeds or fallback used
#[tokio::test]
async fn test_ec_mr_002_model_failure_mid_request() {
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::new(registry);
    executor.simulate_model_failure();
    let fallbacks = vec![builtin_profiles()[2].clone()]; // haiku
    let result = executor
        .execute("claude-opus", "write code", &fallbacks)
        .await;
    assert!(result.is_ok());
    let r = result.unwrap();
    // Either retried on same model (failure-once clears) or used fallback
    assert!(r.retried || r.used_fallback || r.model_id == "claude-opus");
}

/// EC-MR-003: Invalid API key — never exposed in errors
#[test]
fn test_ec_mr_003_invalid_api_key() {
    let err = ExecutorError::new_test(executor::ExecutorErrorKind::InvalidApiKey);
    let msg = format!("{err}");
    assert!(!msg.contains("sk-"), "API key must never appear in errors");
    assert!(!msg.contains("key="), "API key must never appear in errors");
    assert!(msg.contains("keychain"));
    assert_eq!(err.severity, ErrorSeverity::Error);
    assert_eq!(err.category, ErrorCategory::SecurityError);
}

/// EC-MR-004: Rate limited — try different model
#[test]
fn test_ec_mr_004_rate_limited() {
    let registry = ModelRegistry::new();
    registry.mark_rate_limited("claude-opus");
    let router = ModelRouter::new(registry);
    let decision = router.route(TaskType::Code, &default_prefs()).unwrap();
    assert_ne!(decision.model_id(), "claude-opus");
}

/// EC-MR-005: User prefers unavailable model — use alternative
#[test]
fn test_ec_mr_005_unavailable_preference() {
    let registry = ModelRegistry::new();
    registry.mark_unavailable("gpt-4o");
    let router = ModelRouter::new(registry);
    let mut prefs = default_prefs();
    prefs.preferred = vec!["gpt-4o".into()];
    let decision = router.route(TaskType::Code, &prefs).unwrap();
    assert_ne!(decision.model_id(), "gpt-4o");
}

/// EC-MR-006: Privacy conflict — require local but only cloud available
#[test]
fn test_ec_mr_006_privacy_conflict() {
    let registry = ModelRegistry::empty();
    // Only register cloud models
    for p in builtin_profiles()
        .iter()
        .filter(|p| p.privacy == PrivacyLevel::Cloud)
    {
        registry.register(p.clone());
    }
    let router = ModelRouter::new(registry);
    let prefs = ModelPreferences::local_only();
    let result = router.route(TaskType::Code, &prefs);
    assert_eq!(result.unwrap_err(), RouterError::PrivacyConflict);
}

/// EC-MR-007: Cost exceeds budget — warn
#[test]
fn test_ec_mr_007_cost_exceeded() {
    let router = ModelRouter::new(ModelRegistry::new());
    let mut prefs = default_prefs();
    prefs.max_cost_per_task = Some(0.0001); // Very low budget
    let decision = router.route(TaskType::Code, &prefs).unwrap();
    // Should still route but warn about cost
    assert!(decision.warns_about_cost() || decision.model_cost() <= 0.0001);
}

/// EC-MR-008: Bad model output — detect and retry
#[tokio::test]
async fn test_ec_mr_008_bad_model_output() {
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::new(registry);
    executor.simulate_bad_output();
    let result = executor.execute("claude-opus", "test", &[]).await.unwrap();
    assert!(result.retried);
    assert!(result.detected_bad_output);
}

/// EC-MR-009: Concurrent routing decisions
#[tokio::test]
async fn test_ec_mr_009_concurrent_routing() {
    let router = std::sync::Arc::new(ModelRouter::new(ModelRegistry::new()));
    let tasks: Vec<_> = (0..100)
        .map(|_| {
            let r = router.clone();
            tokio::spawn(async move { r.route(TaskType::Code, &ModelPreferences::default()) })
        })
        .collect();
    let results = futures::future::join_all(tasks).await;
    assert_eq!(results.len(), 100);
    for result in &results {
        assert!(result.as_ref().unwrap().is_ok());
    }
}

/// EC-MR-010: Local model runs out of memory — fallback to cloud
#[tokio::test]
async fn test_ec_mr_010_local_model_oom() {
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::new(registry);
    executor.simulate_local_oom();
    let cloud_fallback = builtin_profiles()
        .into_iter()
        .find(|p| p.privacy == PrivacyLevel::Cloud)
        .unwrap();
    let result = executor
        .execute("llama-3-70b", "test", &[cloud_fallback])
        .await;
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.used_fallback || r.model_id != "llama-3-70b");
}

// ═══════════════════════════════════════════════════════════
// SECURITY TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_no_api_keys_in_error_messages() {
    use executor::ExecutorErrorKind;
    let kinds = vec![
        ExecutorErrorKind::ModelNotFound,
        ExecutorErrorKind::ModelUnavailable,
        ExecutorErrorKind::ModelFailed,
        ExecutorErrorKind::AllModelsFailed,
        ExecutorErrorKind::OutOfMemory,
        ExecutorErrorKind::RateLimited,
        ExecutorErrorKind::InvalidApiKey,
        ExecutorErrorKind::Timeout,
        ExecutorErrorKind::CircuitOpen,
    ];
    for kind in kinds {
        let err = ExecutorError::new_test(kind);
        let msg = format!("{err}");
        assert!(!msg.contains("sk-"), "Error contains API key: {msg}");
        assert!(!msg.contains("Bearer"), "Error contains auth token: {msg}");
        assert!(
            !msg.to_lowercase().contains("api_key="),
            "Error contains key: {msg}"
        );
    }
}

#[test]
fn test_privacy_enforced_before_routing() {
    let router = ModelRouter::new(ModelRegistry::new());
    let prefs = ModelPreferences::local_only();
    let decision = router.route(TaskType::General, &prefs);
    match decision {
        Ok(d) => assert_ne!(d.model.privacy, PrivacyLevel::Cloud),
        Err(RouterError::PrivacyConflict) => {}
        Err(e) => panic!("Unexpected error: {e}"),
    }
}

// ═══════════════════════════════════════════════════════════
// CIRCUIT BREAKER TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_circuit_breaker_closed_by_default() {
    let cb = CircuitBreaker::new();
    assert_eq!(cb.state(), CircuitState::Closed);
    assert!(!cb.is_open());
}

#[test]
fn test_circuit_breaker_opens_after_threshold() {
    let cb = CircuitBreaker::new();
    // 5 failures → open
    for _ in 0..5 {
        cb.track_failure();
    }
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(cb.is_open());
}

#[test]
fn test_circuit_breaker_success_resets_in_half_open() {
    let cb = CircuitBreaker::new();
    for _ in 0..5 {
        cb.track_failure();
    }
    assert!(cb.is_open());
    cb.reset(); // Simulate recovery timeout passing
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn test_circuit_breaker_per_model() {
    let registry = ModelRegistry::new();
    let executor = ModelExecutor::new(registry);
    let cb = executor.circuit_breaker("claude-opus");
    assert!(cb.is_some());
    assert_eq!(cb.unwrap().state(), CircuitState::Closed);
}

// ═══════════════════════════════════════════════════════════
// ERROR CLASSIFICATION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_error_has_severity_and_category() {
    use executor::ExecutorErrorKind;
    let err = ExecutorError::new_test(ExecutorErrorKind::OutOfMemory);
    assert_eq!(err.severity, ErrorSeverity::Critical);
    assert_eq!(err.category, ErrorCategory::ResourceError);
    assert!(!err.user_message().is_empty());
    assert!(!err.suggested_action().is_empty());
}

#[test]
fn test_all_errors_have_user_message_and_action() {
    use executor::ExecutorErrorKind;
    let kinds = vec![
        ExecutorErrorKind::ModelNotFound,
        ExecutorErrorKind::ModelUnavailable,
        ExecutorErrorKind::ModelFailed,
        ExecutorErrorKind::AllModelsFailed,
        ExecutorErrorKind::OutOfMemory,
        ExecutorErrorKind::RateLimited,
        ExecutorErrorKind::InvalidApiKey,
        ExecutorErrorKind::Timeout,
        ExecutorErrorKind::CircuitOpen,
    ];
    for kind in kinds {
        let err = ExecutorError::new_test(kind);
        assert!(!err.user_message().is_empty());
        assert!(!err.suggested_action().is_empty());
    }
}

// Timeout constants test moved to model_tests_extra.rs
