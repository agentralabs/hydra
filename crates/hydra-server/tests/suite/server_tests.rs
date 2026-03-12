use std::time::Duration;

use hydra_runtime::jsonrpc::RpcErrorCodes;
use hydra_server::{build_router, handle_rpc};

use super::helpers::{rpc_body, test_state};

// ═══════════════════════════════════════════════════════════
// SERVER LIFECYCLE
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_server_starts() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    server.abort();
}

#[tokio::test]
async fn test_server_stops_gracefully() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    server.abort();
    let result = server.await;
    assert!(result.is_err()); // Aborted = cancelled = err
}

// ═══════════════════════════════════════════════════════════
// HEALTH ENDPOINT
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_health_endpoint() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], "0.1.0");
    assert!(body["uptime_seconds"].is_number());

    server.abort();
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: hydra.run
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_hydra_run() {
    let state = test_state();
    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "list files"})),
    )
    .await;
    assert!(resp.is_success());
    let result = resp.result.unwrap();
    assert!(result["run_id"].is_string());
    assert_eq!(result["status"], "accepted");

    // Verify run was stored in db
    let run_id = result["run_id"].as_str().unwrap();
    let run = state.db.get_run(run_id).unwrap();
    assert_eq!(run.intent, "list files");
}

#[tokio::test]
async fn test_rpc_hydra_run_missing_intent() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.run", serde_json::json!({}))).await;
    assert!(!resp.is_success());
    assert_eq!(resp.error.unwrap().code, RpcErrorCodes::INVALID_PARAMS);
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: hydra.cancel
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_hydra_cancel() {
    let state = test_state();

    // Create a run first
    let run_resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test"})),
    )
    .await;
    let run_id = run_resp.result.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Re-set to running so we can cancel
    state
        .db
        .update_run_status(&run_id, hydra_db::RunStatus::Running, None)
        .unwrap();

    // Cancel
    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.cancel", serde_json::json!({"run_id": run_id})),
    )
    .await;
    assert!(resp.is_success());

    let run = state.db.get_run(&run_id).unwrap();
    assert_eq!(run.status, hydra_db::RunStatus::Cancelled);
}

#[tokio::test]
async fn test_rpc_hydra_cancel_missing_id() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.cancel", serde_json::json!({}))).await;
    assert!(!resp.is_success());
    assert_eq!(resp.error.unwrap().code, RpcErrorCodes::INVALID_PARAMS);
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: hydra.approve
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_hydra_approve() {
    let state = test_state();

    // Create run + approval
    let run_resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test"})),
    )
    .await;
    let run_id = run_resp.result.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    let approval = hydra_db::ApprovalRow {
        id: "a1".into(),
        run_id: run_id.clone(),
        action: "delete_file".into(),
        target: Some("/old.rs".into()),
        risk_score: 0.7,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: chrono::Utc::now().to_rfc3339(),
        status: hydra_db::ApprovalStatus::Pending,
    };
    state.db.create_approval(&approval).unwrap();

    // Approve
    let resp = handle_rpc(
        &state,
        &rpc_body(
            "hydra.approve",
            serde_json::json!({"approval_id": "a1", "decision": "approved"}),
        ),
    )
    .await;
    assert!(resp.is_success());

    let updated = state.db.get_approval("a1").unwrap();
    assert_eq!(updated.status, hydra_db::ApprovalStatus::Approved);
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: hydra.status
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_hydra_status() {
    let state = test_state();

    // No runs
    let resp = handle_rpc(&state, &rpc_body("hydra.status", serde_json::json!({}))).await;
    assert!(resp.is_success());
    let result = resp.result.unwrap();
    assert!(result["runs"].as_array().unwrap().is_empty());

    // Create a run
    handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test"})),
    )
    .await;

    // List all runs
    let resp = handle_rpc(&state, &rpc_body("hydra.status", serde_json::json!({}))).await;
    assert!(resp.is_success());
    let runs = resp.result.unwrap()["runs"].as_array().unwrap().len();
    assert_eq!(runs, 1);
}

#[tokio::test]
async fn test_rpc_hydra_status_specific_run() {
    let state = test_state();

    let run_resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test"})),
    )
    .await;
    let run_id = run_resp.result.unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for async cognitive loop to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.status", serde_json::json!({"run_id": run_id})),
    )
    .await;
    assert!(resp.is_success());
    let runs = resp.result.unwrap()["runs"].as_array().unwrap().clone();
    assert_eq!(runs.len(), 1);
    assert!(runs[0]["steps"].as_array().unwrap().len() > 0);
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: hydra.health
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_hydra_health() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.health", serde_json::json!({}))).await;
    assert!(resp.is_success());
    let result = resp.result.unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result["uptime_seconds"].is_number());
    assert!(result["sisters"].is_object());
}

// ═══════════════════════════════════════════════════════════
// JSON-RPC: ERROR CODES
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_rpc_parse_error() {
    let state = test_state();
    let resp = handle_rpc(&state, "not json{{{").await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::PARSE_ERROR
    );
}

#[tokio::test]
async fn test_rpc_invalid_request() {
    let state = test_state();
    let resp = handle_rpc(
        &state,
        &serde_json::json!({
            "jsonrpc": "1.0",
            "id": "1",
            "method": ""
        })
        .to_string(),
    )
    .await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::INVALID_REQUEST
    );
}

#[tokio::test]
async fn test_rpc_method_not_found() {
    let state = test_state();
    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.nonexistent", serde_json::json!({})),
    )
    .await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::METHOD_NOT_FOUND
    );
}

#[tokio::test]
async fn test_rpc_invalid_params_run() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.run", serde_json::json!({}))).await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::INVALID_PARAMS
    );
}

#[tokio::test]
async fn test_rpc_invalid_params_cancel() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.cancel", serde_json::json!({}))).await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::INVALID_PARAMS
    );
}

#[tokio::test]
async fn test_rpc_invalid_params_approve() {
    let state = test_state();
    let resp = handle_rpc(&state, &rpc_body("hydra.approve", serde_json::json!({}))).await;
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code,
        RpcErrorCodes::INVALID_PARAMS
    );
}

#[tokio::test]
async fn test_rpc_error_all_codes_defined() {
    // Verify all 11 error codes exist
    assert_eq!(RpcErrorCodes::PARSE_ERROR, -32700);
    assert_eq!(RpcErrorCodes::INVALID_REQUEST, -32600);
    assert_eq!(RpcErrorCodes::METHOD_NOT_FOUND, -32601);
    assert_eq!(RpcErrorCodes::INVALID_PARAMS, -32602);
    assert_eq!(RpcErrorCodes::INTERNAL_ERROR, -32603);
    assert_eq!(RpcErrorCodes::SISTER_UNAVAILABLE, -32000);
    assert_eq!(RpcErrorCodes::RUN_FAILED, -32001);
    assert_eq!(RpcErrorCodes::APPROVAL_REQUIRED, -32002);
    assert_eq!(RpcErrorCodes::CAPABILITY_DENIED, -32003);
    assert_eq!(RpcErrorCodes::RESOURCE_EXHAUSTED, -32004);
    assert_eq!(RpcErrorCodes::TIMEOUT, -32005);
}
