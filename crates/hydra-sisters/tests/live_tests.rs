#![cfg(feature = "live-sisters")]

use std::time::Duration;

use hydra_sisters::bridge::*;
use hydra_sisters::live_bridge::{BridgeConfig, LiveMcpBridge};

/// Helper: create a live HTTP bridge for a sister
fn live_bridge(sister_id: SisterId, port: u16) -> LiveMcpBridge {
    let caps = hydra_sisters::bridges::all_bridges()
        .into_iter()
        .find(|b| b.sister_id() == sister_id)
        .map(|b| b.capabilities())
        .unwrap_or_default();

    LiveMcpBridge::http(
        sister_id,
        format!("http://localhost:{}", port),
        caps,
        BridgeConfig::default(),
    )
}

/// Sister port mapping (convention for live testing)
fn port_for(id: SisterId) -> u16 {
    match id {
        SisterId::Memory => 3001,
        SisterId::Vision => 3002,
        SisterId::Codebase => 3003,
        SisterId::Identity => 3004,
        SisterId::Time => 3005,
        SisterId::Contract => 3006,
        SisterId::Comm => 3007,
        SisterId::Planning => 3008,
        SisterId::Cognition => 3009,
        SisterId::Reality => 3010,
        SisterId::Forge => 3011,
        SisterId::Aegis => 3012,
        SisterId::Veritas => 3013,
        SisterId::Evolve => 3014,
    }
}

// ═══════════════════════════════════════════════════════════
// CATEGORY A: CONNECTION TESTS (14 tests)
// ═══════════════════════════════════════════════════════════

macro_rules! connection_test {
    ($name:ident, $sister:expr) => {
        #[tokio::test]
        async fn $name() {
            let bridge = live_bridge($sister, port_for($sister));
            let health = bridge.health_check().await;
            assert_eq!(
                health,
                HealthStatus::Healthy,
                "{} should be healthy",
                $sister.name()
            );
        }
    };
}

connection_test!(test_connection_memory, SisterId::Memory);
connection_test!(test_connection_vision, SisterId::Vision);
connection_test!(test_connection_codebase, SisterId::Codebase);
connection_test!(test_connection_identity, SisterId::Identity);
connection_test!(test_connection_time, SisterId::Time);
connection_test!(test_connection_contract, SisterId::Contract);
connection_test!(test_connection_comm, SisterId::Comm);
connection_test!(test_connection_planning, SisterId::Planning);
connection_test!(test_connection_cognition, SisterId::Cognition);
connection_test!(test_connection_reality, SisterId::Reality);
connection_test!(test_connection_forge, SisterId::Forge);
connection_test!(test_connection_aegis, SisterId::Aegis);
connection_test!(test_connection_veritas, SisterId::Veritas);
connection_test!(test_connection_evolve, SisterId::Evolve);

// ═══════════════════════════════════════════════════════════
// CATEGORY B: TOOL EXECUTION TESTS (28 tests — 2 per sister)
// ═══════════════════════════════════════════════════════════

// Memory: memory_add + memory_query
#[tokio::test]
async fn test_memory_add() {
    let bridge = live_bridge(SisterId::Memory, port_for(SisterId::Memory));
    let result = bridge
        .call(SisterAction::new(
            "memory_add",
            serde_json::json!({
                "content": "Hydra live integration test memory",
                "context": "live_test",
            }),
        ))
        .await;
    assert!(result.is_ok(), "memory_add failed: {:?}", result.err());
}

#[tokio::test]
async fn test_memory_query() {
    let bridge = live_bridge(SisterId::Memory, port_for(SisterId::Memory));
    let result = bridge
        .call(SisterAction::new(
            "memory_query",
            serde_json::json!({
                "query": "integration test",
                "limit": 5,
            }),
        ))
        .await;
    assert!(result.is_ok(), "memory_query failed: {:?}", result.err());
}

// Vision: vision_capture + vision_diff
#[tokio::test]
async fn test_vision_capture() {
    let bridge = live_bridge(SisterId::Vision, port_for(SisterId::Vision));
    let result = bridge
        .call(SisterAction::new(
            "vision_capture",
            serde_json::json!({
                "source": "screen",
            }),
        ))
        .await;
    assert!(result.is_ok(), "vision_capture failed: {:?}", result.err());
}

#[tokio::test]
async fn test_vision_diff() {
    let bridge = live_bridge(SisterId::Vision, port_for(SisterId::Vision));
    let result = bridge
        .call(SisterAction::new(
            "vision_diff",
            serde_json::json!({
                "before": "test_a",
                "after": "test_b",
            }),
        ))
        .await;
    assert!(result.is_ok(), "vision_diff failed: {:?}", result.err());
}

// Codebase: codebase_core + concept_find
#[tokio::test]
async fn test_codebase_core() {
    let bridge = live_bridge(SisterId::Codebase, port_for(SisterId::Codebase));
    let result = bridge
        .call(SisterAction::new(
            "search_semantic",
            serde_json::json!({
                "path": ".",
            }),
        ))
        .await;
    assert!(result.is_ok(), "codebase_core failed: {:?}", result.err());
}

#[tokio::test]
async fn test_codebase_concept_find() {
    let bridge = live_bridge(SisterId::Codebase, port_for(SisterId::Codebase));
    let result = bridge
        .call(SisterAction::new(
            "concept_find",
            serde_json::json!({
                "query": "main function",
            }),
        ))
        .await;
    assert!(result.is_ok(), "concept_find failed: {:?}", result.err());
}

// Identity: identity_create + action_sign
#[tokio::test]
async fn test_identity_create() {
    let bridge = live_bridge(SisterId::Identity, port_for(SisterId::Identity));
    let result = bridge
        .call(SisterAction::new(
            "identity_create",
            serde_json::json!({
                "name": "hydra-live-test",
            }),
        ))
        .await;
    assert!(result.is_ok(), "identity_create failed: {:?}", result.err());
}

#[tokio::test]
async fn test_identity_action_sign() {
    let bridge = live_bridge(SisterId::Identity, port_for(SisterId::Identity));
    let result = bridge
        .call(SisterAction::new(
            "action_sign",
            serde_json::json!({
                "action_type": "test",
                "data": "integration test payload",
            }),
        ))
        .await;
    assert!(result.is_ok(), "action_sign failed: {:?}", result.err());
}

// Time: time_deadline_add + time_schedule_create
#[tokio::test]
async fn test_time_deadline_add() {
    let bridge = live_bridge(SisterId::Time, port_for(SisterId::Time));
    let result = bridge
        .call(SisterAction::new(
            "time_deadline_add",
            serde_json::json!({
                "name": "test-deadline",
                "due": "2030-01-01T00:00:00Z",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "time_deadline_add failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_time_schedule_create() {
    let bridge = live_bridge(SisterId::Time, port_for(SisterId::Time));
    let result = bridge
        .call(SisterAction::new(
            "time_schedule_create",
            serde_json::json!({
                "name": "test-schedule",
                "cron": "0 0 * * *",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "time_schedule_create failed: {:?}",
        result.err()
    );
}

// Contract: contract_create + policy_check
#[tokio::test]
async fn test_contract_create() {
    let bridge = live_bridge(SisterId::Contract, port_for(SisterId::Contract));
    let result = bridge
        .call(SisterAction::new(
            "contract_create",
            serde_json::json!({
                "name": "test-contract",
                "terms": ["no destructive actions"],
            }),
        ))
        .await;
    assert!(result.is_ok(), "contract_create failed: {:?}", result.err());
}

#[tokio::test]
async fn test_contract_policy_check() {
    let bridge = live_bridge(SisterId::Contract, port_for(SisterId::Contract));
    let result = bridge
        .call(SisterAction::new(
            "policy_check",
            serde_json::json!({
                "action": "file_delete",
                "path": "/etc/passwd",
            }),
        ))
        .await;
    assert!(result.is_ok(), "policy_check failed: {:?}", result.err());
}

// Comm: comm_channel + comm_message
#[tokio::test]
async fn test_comm_channel() {
    let bridge = live_bridge(SisterId::Comm, port_for(SisterId::Comm));
    let result = bridge
        .call(SisterAction::new(
            "comm_channel",
            serde_json::json!({
                "operation": "list",
            }),
        ))
        .await;
    assert!(result.is_ok(), "comm_channel failed: {:?}", result.err());
}

#[tokio::test]
async fn test_comm_message() {
    let bridge = live_bridge(SisterId::Comm, port_for(SisterId::Comm));
    let result = bridge
        .call(SisterAction::new(
            "comm_message",
            serde_json::json!({
                "operation": "send",
                "channel": "test",
                "content": "integration test",
            }),
        ))
        .await;
    assert!(result.is_ok(), "comm_message failed: {:?}", result.err());
}

// Planning: planning_goal + planning_decision
#[tokio::test]
async fn test_planning_goal() {
    let bridge = live_bridge(SisterId::Planning, port_for(SisterId::Planning));
    let result = bridge
        .call(SisterAction::new(
            "planning_goal",
            serde_json::json!({
                "operation": "create",
                "name": "test goal",
            }),
        ))
        .await;
    assert!(result.is_ok(), "planning_goal failed: {:?}", result.err());
}

#[tokio::test]
async fn test_planning_decision() {
    let bridge = live_bridge(SisterId::Planning, port_for(SisterId::Planning));
    let result = bridge
        .call(SisterAction::new(
            "planning_decision",
            serde_json::json!({
                "operation": "create",
                "question": "should we proceed?",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "planning_decision failed: {:?}",
        result.err()
    );
}

// Cognition: cognition_model_create + cognition_predict
#[tokio::test]
async fn test_cognition_model_create() {
    let bridge = live_bridge(SisterId::Cognition, port_for(SisterId::Cognition));
    let result = bridge
        .call(SisterAction::new(
            "cognition_model_create",
            serde_json::json!({
                "user_id": "test-user",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "cognition_model_create failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_cognition_predict() {
    let bridge = live_bridge(SisterId::Cognition, port_for(SisterId::Cognition));
    let result = bridge
        .call(SisterAction::new(
            "cognition_predict",
            serde_json::json!({
                "context": "test prediction",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "cognition_predict failed: {:?}",
        result.err()
    );
}

// Reality: reality_deployment + reality_verify
#[tokio::test]
async fn test_reality_deployment() {
    let bridge = live_bridge(SisterId::Reality, port_for(SisterId::Reality));
    let result = bridge
        .call(SisterAction::new(
            "reality_deployment",
            serde_json::json!({
                "operation": "status",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "reality_deployment failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_reality_verify() {
    let bridge = live_bridge(SisterId::Reality, port_for(SisterId::Reality));
    let result = bridge
        .call(SisterAction::new(
            "reality_verify",
            serde_json::json!({
                "claim": "test claim to verify",
            }),
        ))
        .await;
    assert!(result.is_ok(), "reality_verify failed: {:?}", result.err());
}

// Forge: forge_blueprint_create + forge_structure_generate
#[tokio::test]
async fn test_forge_blueprint_create() {
    let bridge = live_bridge(SisterId::Forge, port_for(SisterId::Forge));
    let result = bridge
        .call(SisterAction::new(
            "forge_blueprint_create",
            serde_json::json!({
                "name": "test-blueprint",
                "description": "Integration test blueprint",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "forge_blueprint_create failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_forge_structure_generate() {
    let bridge = live_bridge(SisterId::Forge, port_for(SisterId::Forge));
    let result = bridge
        .call(SisterAction::new(
            "forge_structure_generate",
            serde_json::json!({
                "blueprint_id": "test",
                "target": "rust",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "forge_structure_generate failed: {:?}",
        result.err()
    );
}

// Aegis: aegis_validate_streaming + aegis_shadow_execute
#[tokio::test]
async fn test_aegis_validate_streaming() {
    let bridge = live_bridge(SisterId::Aegis, port_for(SisterId::Aegis));
    let result = bridge
        .call(SisterAction::new(
            "aegis_validate_streaming",
            serde_json::json!({
                "action": "file_write",
                "path": "/tmp/test.txt",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "aegis_validate_streaming failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_aegis_shadow_execute() {
    let bridge = live_bridge(SisterId::Aegis, port_for(SisterId::Aegis));
    let result = bridge
        .call(SisterAction::new(
            "aegis_shadow_execute",
            serde_json::json!({
                "command": "echo test",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "aegis_shadow_execute failed: {:?}",
        result.err()
    );
}

// Veritas: veritas_compile_intent + veritas_verify_claim
#[tokio::test]
async fn test_veritas_compile_intent() {
    let bridge = live_bridge(SisterId::Veritas, port_for(SisterId::Veritas));
    let result = bridge
        .call(SisterAction::new(
            "veritas_compile_intent",
            serde_json::json!({
                "intent": "deploy the application to production",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "veritas_compile_intent failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_veritas_verify_claim() {
    let bridge = live_bridge(SisterId::Veritas, port_for(SisterId::Veritas));
    let result = bridge
        .call(SisterAction::new(
            "veritas_verify_claim",
            serde_json::json!({
                "claim": "all tests pass",
                "evidence": "cargo test output shows 0 failures",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "veritas_verify_claim failed: {:?}",
        result.err()
    );
}

// Evolve: evolve_pattern_store + evolve_crystallize
#[tokio::test]
async fn test_evolve_pattern_store() {
    let bridge = live_bridge(SisterId::Evolve, port_for(SisterId::Evolve));
    let result = bridge
        .call(SisterAction::new(
            "evolve_pattern_store",
            serde_json::json!({
                "pattern": "error-retry",
                "description": "Retry on transient errors",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "evolve_pattern_store failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_evolve_crystallize() {
    let bridge = live_bridge(SisterId::Evolve, port_for(SisterId::Evolve));
    let result = bridge
        .call(SisterAction::new(
            "evolve_crystallize",
            serde_json::json!({
                "pattern_id": "test-pattern",
            }),
        ))
        .await;
    assert!(
        result.is_ok(),
        "evolve_crystallize failed: {:?}",
        result.err()
    );
}

// ═══════════════════════════════════════════════════════════
// CATEGORY C: ERROR HANDLING TESTS (14 tests)
// ═══════════════════════════════════════════════════════════

macro_rules! error_test {
    ($name:ident, $sister:expr, $tool:expr) => {
        #[tokio::test]
        async fn $name() {
            let bridge = live_bridge($sister, port_for($sister));
            // Send invalid/empty params to a real tool
            let result = bridge.call(SisterAction::new($tool, serde_json::json!({
                "__invalid_param__": true,
            }))).await;
            // Should get an error response (not a crash/timeout)
            // The tool should reject invalid params gracefully
            assert!(
                result.is_err() || result.as_ref().unwrap().data.get("error").is_some(),
                "{} should handle invalid params gracefully for {}",
                $sister.name(), $tool
            );
        }
    };
}

error_test!(test_error_memory, SisterId::Memory, "memory_add");
error_test!(test_error_vision, SisterId::Vision, "vision_capture");
error_test!(test_error_codebase, SisterId::Codebase, "search_semantic");
error_test!(test_error_identity, SisterId::Identity, "identity_create");
error_test!(test_error_time, SisterId::Time, "time_deadline_add");
error_test!(test_error_contract, SisterId::Contract, "contract_create");
error_test!(test_error_comm, SisterId::Comm, "comm_channel");
error_test!(test_error_planning, SisterId::Planning, "planning_goal");
error_test!(
    test_error_cognition,
    SisterId::Cognition,
    "cognition_model_create"
);
error_test!(test_error_reality, SisterId::Reality, "reality_deployment");
error_test!(test_error_forge, SisterId::Forge, "forge_blueprint_create");
error_test!(
    test_error_aegis,
    SisterId::Aegis,
    "aegis_validate_streaming"
);
error_test!(
    test_error_veritas,
    SisterId::Veritas,
    "veritas_compile_intent"
);
error_test!(test_error_evolve, SisterId::Evolve, "evolve_pattern_store");

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
