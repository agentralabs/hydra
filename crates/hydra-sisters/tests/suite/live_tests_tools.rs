use hydra_sisters::bridge::*;
use hydra_sisters::live_bridge::{BridgeConfig, LiveMcpBridge};

use super::live_helpers::{live_bridge, port_for};

// ═══════════════════════════════════════════════════════════
// CATEGORY B: TOOL EXECUTION TESTS — Planning through Evolve
// ═══════════════════════════════════════════════════════════

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
