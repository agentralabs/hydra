use hydra_core::types::CognitivePhase;
use hydra_kernel::cognitive_loop::{CycleInput, PhaseHandler};
use hydra_model::LlmConfig;
use hydra_runtime::cognitive::{
    loop_impl::{parse_json_with_fallback, CognitiveLoopConfig, LlmPhaseHandler},
    prompts,
    types::*,
};

// ═══════════════════════════════════════════════════════════
// TYPE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_perception_types() {
    let p = Perception {
        intent: "Create a sort function".into(),
        intent_type: "code_generation".into(),
        entities: vec![Entity {
            entity_type: "language".into(),
            value: "Python".into(),
        }],
        implicit_context: vec!["user prefers clean code".into()],
        urgency: Urgency::Medium,
        required_sisters: vec!["codebase".into(), "forge".into()],
    };
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["intent"], "Create a sort function");
    assert_eq!(json["urgency"], "medium");
    assert_eq!(json["entities"][0]["type"], "language");
    assert_eq!(p.required_sisters.len(), 2);
}

#[test]
fn test_thinking_types() {
    let t = ThinkingResult {
        reasoning: "Need to implement bubble sort".into(),
        steps: vec!["Define function".into(), "Implement loop".into()],
        missing_info: vec![],
        risks: vec!["Performance on large lists".into()],
        confidence: 0.85,
    };
    let json = serde_json::to_value(&t).unwrap();
    assert_eq!(json["steps"].as_array().unwrap().len(), 2);
    assert_eq!(json["confidence"], 0.85);
}

#[test]
fn test_decision_types() {
    let d = Decision {
        action: "generate_code".into(),
        rationale: "Direct code generation is appropriate".into(),
        target: Some("sort.py".into()),
        fallback: Some("suggest pseudocode".into()),
        reversible: true,
    };
    let json = serde_json::to_value(&d).unwrap();
    assert_eq!(json["action"], "generate_code");
    assert!(json["reversible"].as_bool().unwrap());
}

#[test]
fn test_learning_result_types() {
    let l = LearningResult {
        summary: "Successfully generated sort function".into(),
        extracted_knowledge: vec![],
        patterns_observed: vec!["User prefers Python".into()],
        should_remember: true,
    };
    let json = serde_json::to_value(&l).unwrap();
    assert!(json["should_remember"].as_bool().unwrap());
}

#[test]
fn test_cognitive_loop_creation() {
    let handler = LlmPhaseHandler::with_defaults();
    assert_eq!(handler.total_tokens(), 0);
    assert!(handler.phase_metrics().is_empty());
}

#[test]
fn test_cognitive_loop_config_defaults() {
    let config = CognitiveLoopConfig::default();
    assert_eq!(config.perception_model, "claude-haiku");
    assert_eq!(config.thinking_model, "claude-sonnet");
    assert_eq!(config.decision_model, "claude-haiku");
}

// ═══════════════════════════════════════════════════════════
// PROMPT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_prompts_not_empty() {
    assert!(!prompts::perceive_system_prompt().is_empty());
    assert!(!prompts::think_system_prompt().is_empty());
    assert!(!prompts::decide_system_prompt().is_empty());
    assert!(!prompts::learn_system_prompt().is_empty());

    // Each prompt should mention JSON
    assert!(prompts::perceive_system_prompt().contains("JSON"));
    assert!(prompts::think_system_prompt().contains("JSON"));
    assert!(prompts::decide_system_prompt().contains("JSON"));
    assert!(prompts::learn_system_prompt().contains("JSON"));
}

// ═══════════════════════════════════════════════════════════
// SSE EVENT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_phase_event_serialization() {
    let event = CognitivePhaseEvent {
        phase: CognitivePhase::Perceive,
        status: PhaseEventStatus::Completed,
        tokens_used: 150,
        duration_ms: 320,
        data: serde_json::json!({"intent": "test"}),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("perceive"));
    assert!(json.contains("completed"));
    assert!(json.contains("150"));

    // Round-trip
    let parsed: CognitivePhaseEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.phase, CognitivePhase::Perceive);
    assert_eq!(parsed.status, PhaseEventStatus::Completed);
    assert_eq!(parsed.tokens_used, 150);
}

// ═══════════════════════════════════════════════════════════
// JSON PARSING TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_json_parsing_with_fallback() {
    // Valid JSON
    let p: Perception = parse_json_with_fallback(
        r#"{"intent":"test","intent_type":"question","entities":[],"implicit_context":[],"urgency":"high","required_sisters":[]}"#,
    );
    assert_eq!(p.intent, "test");
    assert_eq!(p.urgency, Urgency::High);

    // JSON in markdown code block
    let p: Perception = parse_json_with_fallback(
        "```json\n{\"intent\":\"hello\",\"intent_type\":\"greeting\",\"entities\":[],\"implicit_context\":[],\"urgency\":\"low\",\"required_sisters\":[]}\n```"
    );
    assert_eq!(p.intent, "hello");

    // Invalid JSON → default
    let p: Perception = parse_json_with_fallback("not json at all");
    assert_eq!(p.intent, ""); // default
    assert_eq!(p.urgency, Urgency::Medium); // default
}

// ═══════════════════════════════════════════════════════════
// MOCK EXECUTION TESTS (no API key needed)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mock_perceive_phase() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let handler = LlmPhaseHandler::with_llm_config(config);
    let input = CycleInput::simple("Write a hello world program");

    let result = handler.perceive(&input).await;
    assert!(result.is_ok());
    let val = result.unwrap();
    // Mock fallback fills in the input text as intent
    assert!(val.get("intent").is_some());
}

#[tokio::test]
async fn test_mock_full_phase_chain() {
    let config = LlmConfig {
        anthropic_api_key: None,
        openai_api_key: None,
        anthropic_base_url: "https://api.anthropic.com".into(),
        openai_base_url: "https://api.openai.com".into(),
    };
    let handler = LlmPhaseHandler::with_llm_config(config);
    let input = CycleInput::simple("Fix the bug in main.rs");

    // Run all phases sequentially
    let perceived = handler.perceive(&input).await.unwrap();
    let thought = handler.think(&perceived).await.unwrap();
    let decided = handler.decide(&thought).await.unwrap();
    let _risk = handler.assess_risk(&decided).await.unwrap();
    let acted = handler.act(&decided).await.unwrap();
    let learned = handler.learn(&acted).await.unwrap();

    assert!(learned.get("learned").is_some());
    // Token tracking should have entries for each phase
    assert!(handler.phase_metrics().len() >= 3);
}

// ═══════════════════════════════════════════════════════════
// LIVE LLM TESTS (feature-gated)
// ═══════════════════════════════════════════════════════════

#[cfg(feature = "live-llm")]
mod live {
    use super::*;

    fn live_handler() -> Option<LlmPhaseHandler> {
        let config = LlmConfig::from_env();
        if !config.has_anthropic() && !config.has_openai() {
            eprintln!("Skipping: no API keys set");
            return None;
        }
        Some(LlmPhaseHandler::with_llm_config(config))
    }

    #[tokio::test]
    async fn test_live_perceive() {
        let Some(handler) = live_handler() else {
            return;
        };
        let input = CycleInput::simple("Create a Python function that sorts a list");

        let result = handler.perceive(&input).await.unwrap();
        let intent = result.get("intent").and_then(|v| v.as_str()).unwrap_or("");
        assert!(!intent.is_empty());
        assert!(handler.total_tokens() > 0);
    }

    #[tokio::test]
    async fn test_live_think() {
        let Some(handler) = live_handler() else {
            return;
        };

        let perception = serde_json::json!({
            "intent": "Create a sort function",
            "intent_type": "code_generation",
            "entities": [{"type": "language", "value": "Python"}],
            "implicit_context": [],
            "urgency": "medium",
            "required_sisters": ["codebase"]
        });

        let result = handler.think(&perception).await.unwrap();
        assert!(result.get("reasoning").is_some() || result.get("steps").is_some());
        assert!(handler.total_tokens() > 0);
    }

    #[tokio::test]
    async fn test_live_full_loop() {
        let Some(handler) = live_handler() else {
            return;
        };
        let input = CycleInput::simple("What time is it?");

        let perceived = handler.perceive(&input).await.unwrap();
        let thought = handler.think(&perceived).await.unwrap();
        let decided = handler.decide(&thought).await.unwrap();
        let risk = handler.assess_risk(&decided).await.unwrap();
        let acted = handler.act(&decided).await.unwrap();
        let _learned = handler.learn(&acted).await.unwrap();

        assert!(handler.total_tokens() > 0);
        assert!(handler.phase_metrics().len() >= 3);
        // Risk should be low for a simple question
        assert!(risk.level <= hydra_core::types::RiskLevel::Medium);
    }
}
