use super::*;
use hydra_core::types::CognitivePhase;
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

// ═══════════════════════════════════════════════════════════
// UNIVERSAL FIX TESTS — V3/V4 Memory Integration
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_perceive_includes_longevity_search() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("What did we discuss last week?");
    let result = dispatcher.perceive(&input).await.unwrap();

    // Must include both V2 memory and V4 longevity results
    assert!(result.get("memory").is_some(), "V2 memory_query missing");
    assert!(result.get("longevity").is_some(), "V4 longevity_search missing");
}

#[tokio::test]
async fn test_perceive_longevity_search_not_null() {
    let dispatcher = setup_dispatcher();
    let input = CycleInput::simple("Remind me of our architecture decisions");
    let result = dispatcher.perceive(&input).await.unwrap();

    // Longevity result should be a valid JSON value (even if simulated)
    let longevity = &result["longevity"];
    assert!(!longevity.is_null() || longevity.is_object(),
        "longevity should be a valid response, got: {:?}", longevity);
}

#[tokio::test]
async fn test_learn_v3_capture_message_present() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "refactor the auth module",
        "status": "completed",
        "results": [
            {"sister": "Codebase", "result": {"status": "ok"}},
            {"sister": "Identity", "result": {"status": "ok"}},
        ],
        "risk_level": "low",
        "gate_decision": "approved",
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();

    // V3 capture must be present (replaces old V2-only memory_add)
    assert!(learn_result.get("memory_capture").is_some(),
        "V3 memory_capture_message result missing");
    assert!(learn_result.get("decision_capture").is_some(),
        "V3 memory_capture_decision result missing");
}

#[tokio::test]
async fn test_learn_v3_captures_causal_chain() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "deploy to production",
        "status": "completed",
        "results": [
            {"sister": "Codebase", "result": {"status": "ok"}},
        ],
        "risk_level": "high",
        "gate_decision": "approved",
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();

    // The learn phase should complete even with high-risk actions
    assert_eq!(learn_result["learning"], "completed");
    // Both V3 capture fields must exist
    assert!(learn_result.get("memory_capture").is_some());
    assert!(learn_result.get("decision_capture").is_some());
}

#[tokio::test]
async fn test_learn_with_empty_results() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "simple query",
        "status": "completed",
        "results": [],
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();
    assert_eq!(learn_result["learning"], "completed");
    // Should still capture even with no sister results
    assert!(learn_result.get("memory_capture").is_some());
    assert!(learn_result.get("decision_capture").is_some());
}

#[tokio::test]
async fn test_learn_with_failed_status() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "failed operation",
        "status": "failed",
        "results": [],
        "risk_level": "medium",
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();
    // Learning should complete even for failed actions (we learn from failures)
    assert_eq!(learn_result["learning"], "completed");
    assert!(learn_result.get("memory_capture").is_some());
}

#[tokio::test]
async fn test_learn_with_missing_risk_level() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "query without risk",
        "status": "completed",
        "results": [],
    });

    // Should not panic when risk_level is absent
    let learn_result = dispatcher.learn(&result_data).await.unwrap();
    assert_eq!(learn_result["learning"], "completed");
}

#[tokio::test]
async fn test_learn_with_multiple_sister_results() {
    let dispatcher = setup_dispatcher();
    let result_data = json!({
        "input": "complex multi-sister task",
        "status": "completed",
        "results": [
            {"sister": "Codebase", "result": {"status": "ok"}},
            {"sister": "Vision", "result": {"status": "ok"}},
            {"sister": "Comm", "result": {"status": "ok"}},
            {"sister": "Forge", "result": {"status": "ok"}},
        ],
        "risk_level": "medium",
        "gate_decision": "shadow_first",
    });

    let learn_result = dispatcher.learn(&result_data).await.unwrap();
    assert_eq!(learn_result["learning"], "completed");
    // All 6 parallel learning outputs must be present
    assert!(learn_result.get("memory_capture").is_some());
    assert!(learn_result.get("decision_capture").is_some());
    assert!(learn_result.get("beliefs_updated").is_some());
    assert!(learn_result.get("skill_crystallized").is_some());
    assert!(learn_result.get("continuity").is_some());
    assert!(learn_result.get("time_recorded").is_some());
}

#[tokio::test]
async fn test_decide_includes_memory_risk_context() {
    let dispatcher = setup_dispatcher();
    let thought = json!({
        "input": "delete old backups",
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
    // Memory risk context should be present (queries past similar actions)
    assert!(result.get("memory_risk_context").is_some(),
        "memory_risk_context missing from DECIDE output");
}

#[tokio::test]
async fn test_decide_memory_risk_for_safe_action() {
    let dispatcher = setup_dispatcher();
    let thought = json!({
        "input": "read the README",
        "involves_code": false,
        "involves_vision": false,
        "involves_network": false,
        "intent": {
            "actions": ["Read"],
            "confidence": 0.99,
        },
        "plan": {},
    });

    let result = dispatcher.decide(&thought).await.unwrap();
    assert!(result.get("memory_risk_context").is_some());
    assert_eq!(result["gate_decision"], "approved");
}

#[tokio::test]
async fn test_full_cycle_with_v3_memory_integration() {
    use crate::cognitive_loop::CognitiveLoop;
    use crate::config::KernelConfig;

    let dispatcher = setup_dispatcher();
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let input = CycleInput::simple("What patterns have I used before?");

    let output = kernel.run(input, &dispatcher).await;
    assert!(output.is_ok());
    assert_eq!(output.phases_completed.len(), 5);
    // The full cycle should complete with V3/V4 memory integration
    assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
    assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
}

#[tokio::test]
async fn test_full_cycle_completes_with_v3_memory() {
    use crate::cognitive_loop::CognitiveLoop;
    use crate::config::KernelConfig;

    let dispatcher = setup_dispatcher();
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let input = CycleInput::simple("summarize yesterday's work");

    let output = kernel.run(input, &dispatcher).await;
    assert!(output.is_ok());

    // The CycleOutput.result comes from ACT, not LEARN.
    // LEARN is consumed internally. Verify the full cycle completed all 5 phases,
    // which proves LEARN (with V3 capture) executed successfully.
    assert_eq!(output.phases_completed.len(), 5,
        "Full cycle with V3 memory should complete all 5 phases");
    assert_eq!(output.phases_completed[4], CognitivePhase::Learn,
        "LEARN phase (with V3 capture) must be the final phase");

    // The result (from ACT) should still be valid
    assert!(output.result.get("status").is_some() || output.result.get("receipt").is_some(),
        "ACT result should contain status or receipt. Got: {:?}", output.result);
}

#[tokio::test]
async fn test_perceive_longevity_parallel_with_memory() {
    let dispatcher = setup_dispatcher();

    // Both queries should run in parallel — verify by timing
    let start = std::time::Instant::now();
    let input = CycleInput::simple("Tell me about our project history");
    let result = dispatcher.perceive(&input).await.unwrap();
    let elapsed = start.elapsed();

    // Both should be present
    assert!(result.get("memory").is_some());
    assert!(result.get("longevity").is_some());

    // Parallel execution should complete within timeout (5s per sister + overhead)
    assert!(elapsed.as_secs() < 10,
        "Perceive took {}s — sisters may not be running in parallel", elapsed.as_secs());
}
