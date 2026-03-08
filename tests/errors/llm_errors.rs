//! Category 6: Error Paths — LLM errors.

use hydra_model::*;

#[test]
fn test_api_key_missing() {
    let config = llm_config::LlmConfig::default();
    let result = providers::anthropic::AnthropicClient::new(&config);
    assert!(result.is_err());
    match result.unwrap_err() {
        providers::LlmError::NoApiKey => {}
        e => panic!("expected NoApiKey, got {:?}", e),
    }
}

#[test]
fn test_model_not_found_error() {
    let err = executor::ExecutorError::new_test(executor::ExecutorErrorKind::ModelNotFound);
    assert_eq!(err.kind, executor::ExecutorErrorKind::ModelNotFound);
    assert!(!err.user_message().is_empty());
    assert!(!err.suggested_action().is_empty());
}

#[test]
fn test_rate_limited_error() {
    let err = executor::ExecutorError::new_test(executor::ExecutorErrorKind::RateLimited);
    assert!(!err.user_message().is_empty());
    assert!(!err.suggested_action().is_empty());
}

#[test]
fn test_timeout_error() {
    let err = executor::ExecutorError::new_test(executor::ExecutorErrorKind::Timeout);
    assert_eq!(err.kind, executor::ExecutorErrorKind::Timeout);
}

#[test]
fn test_circuit_open_error() {
    let err = executor::ExecutorError::new_test(executor::ExecutorErrorKind::CircuitOpen);
    assert!(!err.user_message().is_empty());
}

#[test]
fn test_invalid_api_key_error() {
    let err = executor::ExecutorError::new_test(executor::ExecutorErrorKind::InvalidApiKey);
    assert!(!err.suggested_action().is_empty());
}
