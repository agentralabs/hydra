use super::*;
use hydra_core::types::{CognitivePhase, RiskLevel};
use hydra_gate::GateConfig;
use hydra_sisters::bridges;

fn setup_dispatcher() -> SisterDispatcher {
    let mut registry = SisterRegistry::new();
    // Register all 14 sisters
    for bridge in bridges::all_bridges() {
        registry.register(bridge);
    }

    SisterDispatcher::new(
        Arc::new(registry),
        Arc::new(IntentCompiler::new()),
        Arc::new(ExecutionGate::new(GateConfig::default())),
    )
}

#[tokio::test]
async fn test_perceive_always_calls_four_sisters() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("What time is it?");
    let result = dispatcher.perceive(&input).await.unwrap();

    // Memory, Time, Cognition, Reality should all be present
    assert!(result.get("memory").is_some());
    assert!(result.get("temporal").is_some());
    assert!(result.get("user_model").is_some());
    assert!(result.get("reality").is_some());
}

#[tokio::test]
async fn test_perceive_code_adds_codebase() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("Fix the bug in src/main.rs");
    let result = dispatcher.perceive(&input).await.unwrap();

    assert_eq!(result["involves_code"], true);
    assert!(result.get("codebase").is_some());
    assert!(!result["codebase"].is_null());
}

#[tokio::test]
async fn test_perceive_vision_adds_vision() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("Take a screenshot of the UI");
    let result = dispatcher.perceive(&input).await.unwrap();

    assert_eq!(result["involves_vision"], true);
    assert!(result.get("vision").is_some());
    assert!(!result["vision"].is_null());
}

#[tokio::test]
async fn test_perceive_simple_query_no_code_or_vision() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("What is the weather?");
    let result = dispatcher.perceive(&input).await.unwrap();

    assert_eq!(result["involves_code"], false);
    assert_eq!(result["involves_vision"], false);
    assert!(result["codebase"].is_null());
    assert!(result["vision"].is_null());
}

#[tokio::test]
async fn test_think_compiles_intent() {
    let dispatcher = setup_dispatcher();
    let perceived = json!({
        "input": "list all files",
        "involves_code": false,
        "involves_vision": false,
        "involves_network": false,
        "memory": {},
        "temporal": {},
        "user_model": {},
        "reality": {},
        "codebase": null,
        "vision": null,
    });

    let result = dispatcher.think(&perceived).await.unwrap();
    assert!(result.get("intent").is_some());
    assert!(result.get("plan").is_some());
    assert!(result.get("veritas").is_some());
    assert!(result.get("beliefs").is_some());
}

#[tokio::test]
async fn test_think_code_generates_blueprint() {
    let dispatcher = setup_dispatcher();
    let perceived = json!({
        "input": "create a REST API endpoint",
        "involves_code": true,
        "involves_vision": false,
        "involves_network": false,
        "memory": {},
        "temporal": {},
        "user_model": {},
        "reality": {},
        "codebase": {"status": "ok"},
        "vision": null,
    });

    let result = dispatcher.think(&perceived).await.unwrap();
    assert!(!result["blueprint"].is_null());
}

#[tokio::test]
async fn test_decide_low_risk_auto_approves() {
    let dispatcher = setup_dispatcher();
    let thought = json!({
        "input": "list all files",
        "involves_code": false,
        "involves_vision": false,
        "involves_network": false,
        "intent": {
            "actions": ["Read"],
            "confidence": 0.95,
        },
        "plan": {},
    });

    let result = dispatcher.decide(&thought).await.unwrap();
    assert_eq!(result["gate_decision"], "approved");
    assert_eq!(result["risk_level"], "none");
}

#[tokio::test]
async fn test_decide_high_risk_requires_approval() {
    let dispatcher = setup_dispatcher();
    let thought = json!({
        "input": "delete all test files",
        "involves_code": true,
        "involves_vision": false,
        "involves_network": false,
        "intent": {
            "actions": ["FileDelete"],
            "confidence": 0.9,
        },
        "plan": {},
    });

    let result = dispatcher.decide(&thought).await.unwrap();
    assert_eq!(result["gate_decision"], "requires_approval");
    assert_eq!(result["risk_level"], "high");
}

#[tokio::test]
async fn test_decide_medium_risk_shadow_first() {
    let dispatcher = setup_dispatcher();
    let thought = json!({
        "input": "run the test suite",
        "involves_code": true,
        "involves_vision": false,
        "involves_network": false,
        "intent": {
            "actions": ["Execute"],
            "confidence": 0.9,
        },
        "plan": {},
    });

    let result = dispatcher.decide(&thought).await.unwrap();
    assert_eq!(result["gate_decision"], "shadow_first");
    assert_eq!(result["risk_level"], "medium");
}

#[tokio::test]
async fn test_act_blocked_when_requires_approval() {
    let dispatcher = setup_dispatcher();
    let decision = json!({
        "input": "delete everything",
        "gate_decision": "requires_approval",
        "risk_level": "high",
        "thought": {},
    });

    let result = dispatcher.act(&decision).await.unwrap();
    assert_eq!(result["status"], "blocked");
}

#[tokio::test]
async fn test_act_executes_when_approved() {
    let dispatcher = setup_dispatcher();
    let decision = json!({
        "input": "list files",
        "gate_decision": "approved",
        "risk_level": "none",
        "thought": {
            "involves_code": false,
            "involves_network": false,
        },
    });

    let result = dispatcher.act(&decision).await.unwrap();
    assert_eq!(result["status"], "completed");
    assert!(result.get("receipt").is_some());
}

#[tokio::test]
async fn test_learn_calls_all_learning_sisters() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "completed task",
        "status": "completed",
        "results": [],
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();
    assert_eq!(learn_result["learning"], "completed");
    assert!(learn_result.get("memory_capture").is_some());
    assert!(learn_result.get("decision_capture").is_some());
    assert!(learn_result.get("beliefs_updated").is_some());
    assert!(learn_result.get("skill_crystallized").is_some());
    assert!(learn_result.get("continuity").is_some());
    assert!(learn_result.get("time_recorded").is_some());
}

#[tokio::test]
async fn test_assess_risk_levels() {
    let dispatcher = setup_dispatcher();

    let none_risk = dispatcher
        .assess_risk(&json!({"risk_level": "none"}))
        .await
        .unwrap();
    assert_eq!(none_risk.level, RiskLevel::None);
    assert!(!none_risk.needs_approval());

    let high_risk = dispatcher
        .assess_risk(&json!({"risk_level": "high"}))
        .await
        .unwrap();
    assert_eq!(high_risk.level, RiskLevel::High);
    assert!(high_risk.needs_approval());

    let critical_risk = dispatcher
        .assess_risk(&json!({"risk_level": "critical"}))
        .await
        .unwrap();
    assert_eq!(critical_risk.level, RiskLevel::Critical);
    assert!(critical_risk.needs_approval());
}

#[tokio::test]
async fn test_involves_code_detection() {
    // With intent category from micro-LLM (preferred path)
    let with_intent = CycleInput {
        text: "fix it".into(),
        context: json!({"intent_category": "code_fix"}),
    };
    assert!(SisterDispatcher::involves_code(&with_intent));

    let no_code_intent = CycleInput {
        text: "fix it".into(),
        context: json!({"intent_category": "greeting"}),
    };
    assert!(!SisterDispatcher::involves_code(&no_code_intent));

    // Fallback: keyword heuristic (no intent available)
    assert!(SisterDispatcher::involves_code(&CycleInput::simple("Fix the bug in src/main.rs")));
    assert!(SisterDispatcher::involves_code(&CycleInput::simple("cargo build")));
    assert!(!SisterDispatcher::involves_code(&CycleInput::simple("What is the weather?")));
}

#[tokio::test]
async fn test_involves_vision_detection() {
    let with_intent = CycleInput {
        text: "go to google.com".into(),
        context: json!({"intent_category": "web_browse"}),
    };
    assert!(SisterDispatcher::involves_vision(&with_intent));

    assert!(SisterDispatcher::involves_vision(&CycleInput::simple("Take a screenshot")));
    assert!(!SisterDispatcher::involves_vision(&CycleInput::simple("List all files")));
}

#[tokio::test]
async fn test_involves_network_detection() {
    let with_intent = CycleInput {
        text: "tell him".into(),
        context: json!({"intent_category": "communicate"}),
    };
    assert!(SisterDispatcher::involves_network(&with_intent));

    assert!(SisterDispatcher::involves_network(&CycleInput::simple("Send an email")));
    assert!(!SisterDispatcher::involves_network(&CycleInput::simple("Read the file")));
}

#[tokio::test]
async fn test_full_cognitive_cycle() {
    use crate::cognitive_loop::CognitiveLoop;
    use crate::config::KernelConfig;

    let dispatcher = setup_dispatcher();
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let input = CycleInput::simple("list all files in the project");

    let output = kernel.run(input, &dispatcher).await;
    assert!(output.is_ok());
    assert_eq!(output.phases_completed.len(), 5);
    assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
    assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
}

#[tokio::test]
async fn test_full_cycle_with_code_task() {
    use crate::cognitive_loop::CognitiveLoop;
    use crate::config::KernelConfig;

    let dispatcher = setup_dispatcher();
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let input = CycleInput::simple("Fix the bug in src/main.rs");

    let output = kernel.run(input, &dispatcher).await;
    assert!(output.is_ok());
}

#[tokio::test]
async fn test_cache_hit_on_repeated_intent() {
    let dispatcher = setup_dispatcher();

    // First call — may use classifier or LLM
    let perceived1 = json!({
        "input": "list files",
        "involves_code": false,
        "involves_vision": false,
        "involves_network": false,
    });
    let result1 = dispatcher.think(&perceived1).await.unwrap();

    // Second call — should use cache (0 tokens)
    let result2 = dispatcher.think(&perceived1).await.unwrap();

    // Both should have intents
    assert!(result1.get("intent").is_some());
    assert!(result2.get("intent").is_some());
}
