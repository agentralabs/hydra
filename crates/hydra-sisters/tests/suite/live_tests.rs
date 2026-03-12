use hydra_sisters::bridge::*;
use hydra_sisters::live_bridge::{BridgeConfig, LiveMcpBridge};

use super::live_helpers::{live_bridge, port_for};

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
// CATEGORY B: TOOL EXECUTION TESTS — Memory through Comm
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
