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
    assert!(caps.contains(&"impact_analyze".to_string()));
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
