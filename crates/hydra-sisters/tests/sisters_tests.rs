use std::sync::Arc;

use hydra_sisters::batcher::SisterBatcher;
use hydra_sisters::bridge::*;
use hydra_sisters::bridges;
use hydra_sisters::registry::SisterRegistry;

fn build_registry() -> SisterRegistry {
    let mut registry = SisterRegistry::new();
    for bridge in bridges::all_bridges() {
        registry.register(bridge);
    }
    registry
}

// ═══════════════════════════════════════════════════════════
// SISTER ID TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_all_14_sisters_defined() {
    assert_eq!(SisterId::all().len(), 14);
}

#[test]
fn test_sister_categories() {
    let foundation: Vec<_> = SisterId::all()
        .iter()
        .filter(|s| s.is_foundation())
        .collect();
    let cognitive: Vec<_> = SisterId::all()
        .iter()
        .filter(|s| s.is_cognitive())
        .collect();
    let astral: Vec<_> = SisterId::all().iter().filter(|s| s.is_astral()).collect();
    assert_eq!(foundation.len(), 7);
    assert_eq!(cognitive.len(), 3);
    assert_eq!(astral.len(), 4);
}

#[test]
fn test_sister_names() {
    assert_eq!(SisterId::Memory.name(), "agentic-memory");
    assert_eq!(SisterId::Forge.name(), "agentic-forge");
    assert_eq!(SisterId::Evolve.name(), "agentic-evolve");
}

// ═══════════════════════════════════════════════════════════
// INDIVIDUAL BRIDGE TESTS (14 tests, one per sister)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_memory_bridge_connects() {
    let bridge = bridges::memory_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Memory);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "memory_add",
            serde_json::json!({"text": "test"}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_vision_bridge_connects() {
    let bridge = bridges::vision_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Vision);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new("vision_capture", serde_json::json!({})))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_codebase_bridge_connects() {
    let bridge = bridges::codebase_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Codebase);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "search_semantic",
            serde_json::json!({"path": "."}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_identity_bridge_connects() {
    let bridge = bridges::identity_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Identity);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new("identity_create", serde_json::json!({})))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_time_bridge_connects() {
    let bridge = bridges::time_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Time);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "time_schedule_create",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_contract_bridge_connects() {
    let bridge = bridges::contract_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Contract);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new("contract_create", serde_json::json!({})))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_comm_bridge_connects() {
    let bridge = bridges::comm_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Comm);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "comm_message",
            serde_json::json!({"operation": "send", "content": "hi"}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_planning_bridge_connects() {
    let bridge = bridges::planning_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Planning);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new("planning_goal", serde_json::json!({})))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cognition_bridge_connects() {
    let bridge = bridges::cognition_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Cognition);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "cognition_model_create",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reality_bridge_connects() {
    let bridge = bridges::reality_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Reality);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "reality_deployment",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_forge_bridge_connects() {
    let bridge = bridges::forge_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Forge);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "forge_blueprint_create",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_aegis_bridge_connects() {
    let bridge = bridges::aegis_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Aegis);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "aegis_validate_streaming",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_veritas_bridge_connects() {
    let bridge = bridges::veritas_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Veritas);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "veritas_compile_intent",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_evolve_bridge_connects() {
    let bridge = bridges::evolve_bridge();
    assert_eq!(bridge.sister_id(), SisterId::Evolve);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
    let result = bridge
        .call(SisterAction::new(
            "evolve_pattern_store",
            serde_json::json!({}),
        ))
        .await;
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════
// REGISTRY TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_registry_all_14_registered() {
    let registry = build_registry();
    assert_eq!(registry.count(), 14);
}

#[tokio::test]
async fn test_registry_get_by_id() {
    let registry = build_registry();
    let memory = registry.get(SisterId::Memory);
    assert!(memory.is_some());
    assert_eq!(memory.unwrap().name(), "agentic-memory");
}

#[tokio::test]
async fn test_health_check_all() {
    let registry = build_registry();
    let health = registry.health_check_all().await;
    assert_eq!(health.len(), 14);
    for (_, status) in &health {
        assert_eq!(*status, HealthStatus::Healthy);
    }
}

#[tokio::test]
async fn test_list_available() {
    let registry = build_registry();
    let available = registry.list_available().await;
    assert_eq!(available.len(), 14);
}

// ═══════════════════════════════════════════════════════════
// BATCHER TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_batch_groups_correctly() {
    let registry = Arc::new(build_registry());
    let mut batcher = SisterBatcher::new(registry);

    batcher.queue(
        SisterId::Memory,
        SisterAction::new("memory_add", serde_json::json!({"a": 1})),
    );
    batcher.queue(
        SisterId::Memory,
        SisterAction::new("memory_add", serde_json::json!({"a": 2})),
    );
    batcher.queue(
        SisterId::Vision,
        SisterAction::new("vision_capture", serde_json::json!({})),
    );

    assert_eq!(batcher.pending_count(), 3);

    let results = batcher.flush_all().await;
    assert_eq!(results.len(), 2); // Two sisters: Memory + Vision
    assert_eq!(results[&SisterId::Memory].len(), 2); // 2 memory calls
    assert_eq!(results[&SisterId::Vision].len(), 1); // 1 vision call
}

#[tokio::test]
async fn test_batch_parallel_execution() {
    let registry = Arc::new(build_registry());
    let mut batcher = SisterBatcher::new(registry);

    // Queue calls to 5 different sisters
    for id in &[
        SisterId::Memory,
        SisterId::Vision,
        SisterId::Codebase,
        SisterId::Identity,
        SisterId::Time,
    ] {
        batcher.queue(*id, SisterAction::new("test", serde_json::json!({})));
    }

    let results = batcher.flush_all().await;
    assert_eq!(results.len(), 5);
    for (_, sister_results) in &results {
        assert_eq!(sister_results.len(), 1);
        assert!(sister_results[0].is_ok());
    }
}

#[tokio::test]
async fn test_batch_saves_tokens() {
    let registry = Arc::new(build_registry());
    let mut batcher = SisterBatcher::new(registry);

    // Queue 10 individual calls
    for i in 0..10 {
        batcher.queue(
            SisterId::Memory,
            SisterAction::new("memory_add", serde_json::json!({"i": i})),
        );
    }

    let _ = batcher.flush_all().await;

    // Batcher should report savings
    assert_eq!(batcher.individual_calls(), 10);
    assert_eq!(batcher.batched_calls(), 1); // 10 calls → 1 batch
    assert!(batcher.tokens_saved() > 0);
}

// ═══════════════════════════════════════════════════════════
// ERROR HANDLING TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sister_unavailable_graceful() {
    let bridge = bridges::memory_bridge();
    bridge.set_available(false);

    assert_eq!(bridge.health_check().await, HealthStatus::Unavailable);
    let result = bridge
        .call(SisterAction::new("memory_add", serde_json::json!({})))
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.retryable);
    assert_eq!(err.sister_id, SisterId::Memory);
}

#[tokio::test]
async fn test_sister_timeout_handling() {
    let mut bridge = bridges::memory_bridge();
    bridge.set_timeout(std::time::Duration::from_millis(1));
    // Normal calls should still succeed (simulated, no real network)
    let result = bridge
        .call(SisterAction::new("memory_add", serde_json::json!({})))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sister_error_format() {
    let err = SisterError {
        sister_id: SisterId::Memory,
        message: "Connection refused".into(),
        retryable: true,
    };
    let display = format!("{err}");
    assert!(display.contains("agentic-memory"));
    assert!(display.contains("Connection refused"));
    assert!(display.contains("temporary"));
}

#[tokio::test]
async fn test_batch_with_unavailable_sister() {
    let registry = Arc::new({
        let mut r = SisterRegistry::new();
        // Only register Memory, not Vision
        r.register(bridges::memory_bridge());
        r
    });
    let mut batcher = SisterBatcher::new(registry);
    batcher.queue(
        SisterId::Memory,
        SisterAction::new("test", serde_json::json!({})),
    );
    batcher.queue(
        SisterId::Vision,
        SisterAction::new("test", serde_json::json!({})),
    );

    let results = batcher.flush_all().await;
    assert!(results[&SisterId::Memory][0].is_ok());
    assert!(results[&SisterId::Vision][0].is_err());
}

// ═══════════════════════════════════════════════════════════
// CAPABILITIES TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_memory_capabilities() {
    let bridge = bridges::memory_bridge();
    let caps = bridge.capabilities();
    assert!(caps.contains(&"memory_add".to_string()));
    assert!(caps.contains(&"memory_query".to_string()));
    assert!(caps.contains(&"memory_similar".to_string()));
    assert!(caps.contains(&"memory_temporal".to_string()));
    assert!(caps.contains(&"memory_ground".to_string()));
    assert!(caps.contains(&"conversation_log".to_string()));
    assert!(caps.contains(&"memory_workspace_create".to_string()));
}

#[test]
fn test_vision_capabilities() {
    let bridge = bridges::vision_bridge();
    let caps = bridge.capabilities();
    assert!(caps.contains(&"vision_capture".to_string()));
    assert!(caps.contains(&"vision_compare".to_string()));
    assert!(caps.contains(&"vision_ocr".to_string()));
    assert!(caps.contains(&"vision_diff".to_string()));
    assert!(caps.contains(&"observation_log".to_string()));
}

#[test]
fn test_codebase_capabilities() {
    let bridge = bridges::codebase_bridge();
    let caps = bridge.capabilities();
    assert!(caps.contains(&"search_semantic".to_string()));
    assert!(caps.contains(&"concept_find".to_string()));
    assert!(caps.contains(&"impact_analysis".to_string()));
    assert!(caps.contains(&"omniscience_search".to_string()));
    assert!(caps.contains(&"analysis_log".to_string()));
}

#[test]
fn test_identity_capabilities() {
    let bridge = bridges::identity_bridge();
    let caps = bridge.capabilities();
    assert!(caps.contains(&"identity_create".to_string()));
    assert!(caps.contains(&"action_sign".to_string()));
    assert!(caps.contains(&"trust_verify".to_string()));
    assert!(caps.contains(&"receipt_verify".to_string()));
    assert!(caps.contains(&"spawn_create".to_string()));
}

#[test]
fn test_all_bridges_have_capabilities() {
    for bridge in bridges::all_bridges() {
        let caps = bridge.capabilities();
        assert!(
            caps.len() >= 9,
            "{} has only {} capabilities (expected >=9)",
            bridge.name(),
            caps.len()
        );
    }
}

#[test]
fn test_all_bridges_have_versions() {
    for bridge in bridges::all_bridges() {
        assert!(
            !bridge.version().is_empty(),
            "{} has no version",
            bridge.name()
        );
    }
}

// ═══════════════════════════════════════════════════════════
// TOOL NAME REALITY TESTS — verify bridge tools match real sisters
// ═══════════════════════════════════════════════════════════

#[test]
fn test_no_invented_tool_names() {
    // These are the old INVENTED tool names that don't exist in any real sister
    let fake_names = [
        "vision_analyze",
        "codebase_search",
        "codebase_analyze",
        "identity_verify",
        "identity_sign",
        "time_schedule",
        "time_remind",
        "time_track",
        "contract_validate",
        "contract_enforce",
        "contract_audit",
        "comm_send",
        "comm_channel_status",
        "planning_create_goal",
        "planning_decompose",
        "planning_status",
        "cognition_model",
        "cognition_adapt",
        "reality_observe",
        "reality_simulate",
        "forge_blueprint",
        "forge_generate",
        "aegis_validate",
        "aegis_scan",
        "veritas_verify",
        "veritas_fact_check",
        "veritas_source",
        "evolve_learn",
        "evolve_adapt",
    ];

    for bridge in bridges::all_bridges() {
        let caps = bridge.capabilities();
        for fake in &fake_names {
            assert!(
                !caps.contains(&fake.to_string()),
                "{} still contains fake tool name '{}'",
                bridge.name(),
                fake
            );
        }
    }
}

#[test]
fn test_static_capabilities_fallback() {
    // Static capabilities work without any live connection — this is the fallback
    for bridge in bridges::all_bridges() {
        let caps = bridge.capabilities();
        assert!(
            !caps.is_empty(),
            "{} should have static capabilities as fallback",
            bridge.name()
        );
    }
}

#[test]
fn test_total_capabilities_count() {
    let total: usize = bridges::all_bridges()
        .iter()
        .map(|b| b.capabilities().len())
        .sum();
    // With real tool names, total should be well above the old ~42
    assert!(
        total >= 180,
        "Total capabilities across all bridges should be >=180, got {}",
        total
    );
}
