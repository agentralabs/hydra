use std::time::Duration;

use hydra_sisters::bridge::*;
use hydra_sisters::live_bridge::{BridgeConfig, LiveMcpBridge};

use super::live_helpers::{live_bridge, port_for};

// ═══════════════════════════════════════════════════════════
// CATEGORY D: CIRCUIT BREAKER INTEGRATION TESTS (14 tests)
// ═══════════════════════════════════════════════════════════

macro_rules! circuit_breaker_test {
    ($name:ident, $sister:expr) => {
        #[tokio::test]
        async fn $name() {
            // Connect to a bad port to force failures
            let bridge = LiveMcpBridge::http(
                $sister,
                "http://localhost:1", // Port 1 — should fail to connect
                vec!["test".into()],
                BridgeConfig {
                    timeout: Duration::from_millis(500),
                    complex_timeout: Duration::from_secs(1),
                    auto_start: false,
                },
            );

            // Force 5 failures to trip the circuit breaker
            for _ in 0..5 {
                let _ = bridge
                    .call(SisterAction::new("test", serde_json::json!({})))
                    .await;
            }

            // Circuit should now be open — calls rejected immediately
            let result = bridge
                .call(SisterAction::new("test", serde_json::json!({})))
                .await;
            assert!(
                result.is_err(),
                "Circuit should be open for {}",
                $sister.name()
            );
            let err = result.unwrap_err();
            assert!(
                err.message.contains("Circuit breaker open"),
                "Expected circuit breaker error for {}, got: {}",
                $sister.name(),
                err.message
            );
        }
    };
}

circuit_breaker_test!(test_circuit_memory, SisterId::Memory);
circuit_breaker_test!(test_circuit_vision, SisterId::Vision);
circuit_breaker_test!(test_circuit_codebase, SisterId::Codebase);
circuit_breaker_test!(test_circuit_identity, SisterId::Identity);
circuit_breaker_test!(test_circuit_time, SisterId::Time);
circuit_breaker_test!(test_circuit_contract, SisterId::Contract);
circuit_breaker_test!(test_circuit_comm, SisterId::Comm);
circuit_breaker_test!(test_circuit_planning, SisterId::Planning);
circuit_breaker_test!(test_circuit_cognition, SisterId::Cognition);
circuit_breaker_test!(test_circuit_reality, SisterId::Reality);
circuit_breaker_test!(test_circuit_forge, SisterId::Forge);
circuit_breaker_test!(test_circuit_aegis, SisterId::Aegis);
circuit_breaker_test!(test_circuit_veritas, SisterId::Veritas);
circuit_breaker_test!(test_circuit_evolve, SisterId::Evolve);

// ═══════════════════════════════════════════════════════════
// CATEGORY E: CROSS-SISTER TESTS (5 tests)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_cross_memory_identity_receipt_signing() {
    let memory = live_bridge(SisterId::Memory, port_for(SisterId::Memory));
    let identity = live_bridge(SisterId::Identity, port_for(SisterId::Identity));

    // Add a memory
    let mem_result = memory
        .call(SisterAction::new(
            "memory_add",
            serde_json::json!({
                "content": "Cross-sister test memory",
                "context": "cross_test",
            }),
        ))
        .await;
    assert!(mem_result.is_ok(), "memory_add failed");

    // Sign it with identity
    let sign_result = identity
        .call(SisterAction::new(
            "action_sign",
            serde_json::json!({
                "action_type": "memory_add",
                "data": "cross-sister test",
            }),
        ))
        .await;
    assert!(sign_result.is_ok(), "action_sign failed");
}

#[tokio::test]
async fn test_cross_vision_codebase_analysis() {
    let vision = live_bridge(SisterId::Vision, port_for(SisterId::Vision));
    let codebase = live_bridge(SisterId::Codebase, port_for(SisterId::Codebase));

    // Capture a view
    let vis_result = vision
        .call(SisterAction::new(
            "vision_capture",
            serde_json::json!({
                "source": "screen",
            }),
        ))
        .await;
    assert!(vis_result.is_ok(), "vision_capture failed");

    // Analyze codebase
    let code_result = codebase
        .call(SisterAction::new(
            "search_semantic",
            serde_json::json!({
                "path": ".",
            }),
        ))
        .await;
    assert!(code_result.is_ok(), "codebase_core failed");
}

#[tokio::test]
async fn test_cross_time_planning_deadline() {
    let time = live_bridge(SisterId::Time, port_for(SisterId::Time));
    let planning = live_bridge(SisterId::Planning, port_for(SisterId::Planning));

    // Create a deadline
    let time_result = time
        .call(SisterAction::new(
            "time_deadline_add",
            serde_json::json!({
                "name": "cross-test-deadline",
                "due": "2030-12-31T23:59:59Z",
            }),
        ))
        .await;
    assert!(time_result.is_ok(), "time_deadline_add failed");

    // Create a goal with that deadline
    let plan_result = planning
        .call(SisterAction::new(
            "planning_goal",
            serde_json::json!({
                "operation": "create",
                "name": "cross-test goal",
                "deadline": "2030-12-31T23:59:59Z",
            }),
        ))
        .await;
    assert!(plan_result.is_ok(), "planning_goal failed");
}

#[tokio::test]
async fn test_cross_contract_identity_trust() {
    let contract = live_bridge(SisterId::Contract, port_for(SisterId::Contract));
    let identity = live_bridge(SisterId::Identity, port_for(SisterId::Identity));

    // Create a contract
    let contract_result = contract
        .call(SisterAction::new(
            "contract_create",
            serde_json::json!({
                "name": "trust-contract",
                "terms": ["must verify identity"],
            }),
        ))
        .await;
    assert!(contract_result.is_ok(), "contract_create failed");

    // Verify trust
    let trust_result = identity
        .call(SisterAction::new(
            "trust_verify",
            serde_json::json!({
                "entity": "hydra-test",
            }),
        ))
        .await;
    assert!(trust_result.is_ok(), "trust_verify failed");
}

#[tokio::test]
async fn test_cross_aegis_veritas_safety() {
    let aegis = live_bridge(SisterId::Aegis, port_for(SisterId::Aegis));
    let veritas = live_bridge(SisterId::Veritas, port_for(SisterId::Veritas));

    // Validate an action
    let aegis_result = aegis
        .call(SisterAction::new(
            "aegis_validate_streaming",
            serde_json::json!({
                "action": "deploy",
                "target": "production",
            }),
        ))
        .await;
    assert!(aegis_result.is_ok(), "aegis_validate_streaming failed");

    // Verify the intent
    let veritas_result = veritas
        .call(SisterAction::new(
            "veritas_compile_intent",
            serde_json::json!({
                "intent": "deploy to production safely",
            }),
        ))
        .await;
    assert!(veritas_result.is_ok(), "veritas_compile_intent failed");
}

// ═══════════════════════════════════════════════════════════
// CATEGORY F: CAPABILITY DISCOVERY TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_discover_capabilities_returns_all_tools() {
    let mut bridge = live_bridge(SisterId::Memory, port_for(SisterId::Memory));
    let caps = bridge.discover_capabilities().await.unwrap();
    assert!(
        caps.len() >= 20,
        "Memory should have >=20 tools, got {}",
        caps.len()
    );
    assert!(caps.contains(&"memory_add".to_string()));
    assert!(caps.contains(&"memory_query".to_string()));
}

#[tokio::test]
async fn test_discover_capabilities_updates_bridge() {
    let mut bridge = live_bridge(SisterId::Vision, port_for(SisterId::Vision));
    let _before = bridge.capabilities().len();
    let discovered = bridge.discover_capabilities().await.unwrap();
    // After discovery, capabilities should reflect real sister tools
    assert_eq!(bridge.capabilities().len(), discovered.len());
    // Discovered may differ from static list
    assert!(
        discovered.len() >= 10,
        "Vision should have >=10 tools, got {}",
        discovered.len()
    );
}
