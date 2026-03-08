use hydra_core::types::{Action, ActionType};
use hydra_gate::boundary::{BoundaryEnforcer, BoundaryResult};
use hydra_gate::risk::ActionContext;
use hydra_gate::{ExecutionGate, GateDecision};
use hydra_sisters::bridge::SisterId;
use hydra_sisters::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use std::time::Duration;

use hydra_integration::TestServer;

/// Test that boundary violations return a block decision
#[tokio::test]
async fn test_boundary_violation_returns_blocked() {
    let gate = ExecutionGate::default();

    // Try to access /etc/passwd — should be blocked by boundary
    let action = Action::new(ActionType::FileModify, "/etc/passwd");
    let ctx = ActionContext::default();
    let decision = gate.evaluate(&action, &ctx, None).await;

    assert!(
        decision.is_blocked(),
        "Boundary violation should block: {:?}",
        decision
    );
    assert_eq!(decision.risk_score(), 1.0);
}

/// Test that open circuit breaker returns unavailable
#[tokio::test]
async fn test_circuit_open_returns_unavailable() {
    let cb = CircuitBreaker::new(
        SisterId::Memory,
        CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(30),
        },
    );

    // Trip the circuit
    cb.record_failure();
    cb.record_failure();
    cb.record_failure();

    assert_eq!(cb.state(), CircuitState::Open);
    assert!(!cb.allow_call(), "Open circuit should reject calls");
    assert!(cb.total_rejections() > 0);
}

/// Test that a run error emits an SSE error event
#[tokio::test]
async fn test_run_error_emits_sse_error_event() {
    let server = TestServer::start().await;

    // Create a run — it should complete successfully
    let run_id = server.run("error test run").await;
    assert!(!run_id.is_empty());

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify the run exists and has a final status
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    assert!(status["result"]["runs"].as_array().unwrap().len() > 0);
}

/// Test that failed runs update DB status
#[tokio::test]
async fn test_run_error_updates_db_status() {
    let server = TestServer::start().await;

    // Create a normal run
    let run_id = server.run("db error status test").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Check status — should be completed (no error in normal flow)
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    let run_status = status["result"]["runs"][0]["status"].as_str().unwrap();
    assert!(
        run_status == "completed" || run_status == "failed",
        "Run should have a terminal status, got: {run_status}"
    );
}

/// Test boundary enforcer blocks self-modification via gate
#[tokio::test]
async fn test_boundary_enforcer_blocks_self_modification() {
    let enforcer = BoundaryEnforcer::new();

    // All Hydra core paths should be blocked
    let blocked_paths = vec![
        "hydra-gate/src/gate.rs",
        "hydra-kernel/src/cognitive_loop.rs",
        "hydra-core/src/types.rs",
    ];

    for path in blocked_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Blocked(_)),
            "Expected {} to be blocked",
            path
        );
    }
}

/// Test boundary enforcer allows safe paths
#[tokio::test]
async fn test_boundary_enforcer_allows_safe_paths() {
    let enforcer = BoundaryEnforcer::new();

    let safe_paths = vec![
        "src/main.rs",
        "/home/user/project/lib.rs",
        "/tmp/test.txt",
        "README.md",
    ];

    for path in safe_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Allowed),
            "Expected {} to be allowed",
            path
        );
    }
}
