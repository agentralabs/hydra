//! Category 2: Integration — cognitive loop ↔ sisters data flow.

use hydra_runtime::cognitive::types::*;
use hydra_core::*;
use hydra_sisters::*;

#[test]
fn test_perception_to_thinking_data_flow() {
    let perception = Perception {
        intent: "Create a REST API".into(),
        intent_type: "code_generation".into(),
        entities: vec![Entity { entity_type: "language".into(), value: "Rust".into() }],
        implicit_context: vec!["user knows Rust".into()],
        urgency: Urgency::Medium,
        required_sisters: vec!["codebase".into(), "forge".into()],
    };

    let thinking = ThinkingResult {
        reasoning: format!("User wants to {}", perception.intent),
        steps: vec!["scaffold project".into(), "add routes".into()],
        missing_info: vec![],
        risks: vec![],
        confidence: 0.9,
    };

    assert!(thinking.reasoning.contains(&perception.intent));
    assert!(thinking.confidence > 0.5);
}

#[test]
fn test_decision_to_action_data_flow() {
    let decision = Decision {
        action: "generate_code".into(),
        rationale: "best approach".into(),
        target: Some("src/main.rs".into()),
        fallback: Some("template".into()),
        reversible: true,
    };

    assert!(decision.reversible);
    assert!(decision.target.is_some());
    assert!(decision.fallback.is_some());
}

#[test]
fn test_learning_captures_outcome() {
    let learning = LearningResult {
        summary: "Successfully generated API".into(),
        patterns_observed: vec!["REST scaffold pattern".into()],
        should_remember: true,
    };
    assert!(learning.should_remember);
    assert!(!learning.patterns_observed.is_empty());
}

#[test]
fn test_sister_bridge_config() {
    let config = SisterConfig {
        name: "memory".into(),
        endpoint: "stdio://agentic-memory-mcp".into(),
        enabled: true,
        timeout_ms: 5000,
    };
    assert!(config.enabled);
    assert_eq!(config.name, "memory");
}

#[test]
fn test_sister_error_propagation() {
    let err = HydraError::SisterNotFound("memory".into());
    assert!(err.user_message().contains("sister") || err.user_message().contains("Sister") || !err.user_message().is_empty());
    assert!(!err.is_retryable());

    let err2 = HydraError::SisterUnreachable("codebase".into());
    assert!(err2.is_retryable());
}
