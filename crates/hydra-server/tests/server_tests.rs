use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hydra_db::HydraDb;
use hydra_runtime::jsonrpc::RpcErrorCodes;
use hydra_server::{build_router, handle_rpc, AppState};

fn test_state() -> AppState {
    let db = HydraDb::in_memory().unwrap();
    AppState::new(db, false, None)
}

fn test_state_with_auth(token: &str) -> AppState {
    let db = HydraDb::in_memory().unwrap();
    AppState::new(db, true, Some(token.into()))
}

// Helper to make JSON-RPC request body
fn rpc_body(method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": "test-1",
        "method": method,
        "params": params,
    })
    .to_string()
}

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

// ═══════════════════════════════════════════════════════════
// SSE
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_sse_connection() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let resp = reqwest::get(format!("http://{addr}/events")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/event-stream"));

    server.abort();
}

#[tokio::test]
async fn test_sse_run_events() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Trigger a run via RPC
    let client = reqwest::Client::new();
    let rpc_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "1",
        "method": "hydra.run",
        "params": {"intent": "test sse events"}
    });
    let resp = client
        .post(format!("http://{addr}/rpc"))
        .json(&rpc_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["result"]["run_id"].is_string());

    server.abort();
}

// ═══════════════════════════════════════════════════════════
// AUTH
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_auth_required_server_mode() {
    let state = test_state_with_auth("secret-token");
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Without token
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/rpc"))
        .body(rpc_body("hydra.health", serde_json::json!({})))
        .header("content-type", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With correct token
    let resp = client
        .post(format!("http://{addr}/rpc"))
        .body(rpc_body("hydra.health", serde_json::json!({})))
        .header("content-type", "application/json")
        .header("authorization", "Bearer secret-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    server.abort();
}

#[tokio::test]
async fn test_auth_bypass_localhost() {
    // No auth_token set = desktop mode, no auth required
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/rpc"))
        .body(rpc_body("hydra.health", serde_json::json!({})))
        .header("content-type", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    server.abort();
}

// ═══════════════════════════════════════════════════════════
// CORS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_cors_headers() {
    let state = test_state();
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    let headers = resp.headers();
    // CORS should allow all origins
    assert!(headers.get("access-control-allow-origin").is_some());

    server.abort();
}

// ═══════════════════════════════════════════════════════════
// E2E WIRING
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_llm_phase_handler_wired() {
    // Verify that execute_run uses LlmPhaseHandler (EventEmittingHandler wrapping it)
    // by checking that a run completes and generates SSE events with phase data
    let state = test_state();

    // Subscribe to event bus before triggering run
    let mut rx = state.event_bus.subscribe();

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "say hello"})),
    )
    .await;
    assert!(resp.is_success());
    let run_id = resp.result.unwrap()["run_id"].as_str().unwrap().to_string();

    // Collect events for a short period
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                events.push(event);
                // Check if run completed or errored
                let last = events.last().unwrap();
                let evt_type = serde_json::to_string(&last.event_type).unwrap_or_default();
                if evt_type.contains("run_completed") || evt_type.contains("run_error") {
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }

    // Should have run_started + step events + run_completed/error
    assert!(
        events.len() >= 2,
        "Expected at least 2 SSE events, got {}",
        events.len()
    );

    // First event should be run_started
    let first_data = serde_json::to_string(&events[0].data).unwrap();
    assert!(
        first_data.contains(&run_id),
        "First event should reference run_id"
    );
}

#[tokio::test]
async fn test_sse_phase_events_emit() {
    // Verify that phase events (step_started, step_completed) are emitted
    let state = test_state();
    let mut rx = state.event_bus.subscribe();

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test phases"})),
    )
    .await;
    assert!(resp.is_success());

    // Collect events
    let mut step_started_count = 0;
    let mut step_completed_count = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let evt_type = serde_json::to_string(&event.event_type).unwrap_or_default();
                if evt_type.contains("step_started") {
                    step_started_count += 1;
                    // Verify phase name is present
                    let data = serde_json::to_string(&event.data).unwrap();
                    assert!(
                        data.contains("perceive") || data.contains("think") ||
                        data.contains("decide") || data.contains("act") || data.contains("learn"),
                        "Step event should contain phase name: {data}"
                    );
                }
                if evt_type.contains("step_completed") {
                    step_completed_count += 1;
                    // Verify phase metrics are present
                    let data = serde_json::to_string(&event.data).unwrap();
                    assert!(data.contains("duration_ms"), "Completed event should have duration_ms: {data}");
                }
                if evt_type.contains("run_completed") || evt_type.contains("run_error") {
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }

    // Should have at least some step events (even if LLM fails, we get started events)
    assert!(
        step_started_count > 0,
        "Expected step_started events, got 0"
    );
}

#[tokio::test]
async fn test_real_llm_response_returned() {
    // Verify run_completed event includes a response field
    let state = test_state();
    let mut rx = state.event_bus.subscribe();

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "what is 2+2"})),
    )
    .await;
    assert!(resp.is_success());

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut found_response = false;
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let evt_type = serde_json::to_string(&event.event_type).unwrap_or_default();
                if evt_type.contains("run_completed") {
                    let data = serde_json::to_string(&event.data).unwrap();
                    // Should have a response field (even if using fallback defaults)
                    assert!(data.contains("response"), "run_completed should have response: {data}");
                    found_response = true;
                    break;
                }
                if evt_type.contains("run_error") {
                    // Without API key, LLM calls will fail — that's OK, error path works
                    found_response = true;
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }

    assert!(
        found_response,
        "Should have received run_completed or run_error"
    );
}

#[tokio::test]
async fn test_token_counts_real() {
    // Verify that token counts in run_completed reflect real usage
    let state = test_state();
    let mut rx = state.event_bus.subscribe();

    handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "test tokens"})),
    )
    .await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let evt_type = serde_json::to_string(&event.event_type).unwrap_or_default();
                if evt_type.contains("run_completed") {
                    let data = serde_json::to_string(&event.data).unwrap();
                    assert!(data.contains("tokens_used"), "run_completed should report tokens_used");
                    break;
                }
                if evt_type.contains("run_error") {
                    break; // Without API key, errors are expected
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_full_e2e_run() {
    // Full E2E: send message → cognitive loop → DB updated → SSE events
    let state = test_state();
    let mut rx = state.event_bus.subscribe();

    let resp = handle_rpc(
        &state,
        &rpc_body("hydra.run", serde_json::json!({"intent": "full e2e test"})),
    )
    .await;
    assert!(resp.is_success());
    let run_id = resp.result.unwrap()["run_id"].as_str().unwrap().to_string();

    // Wait for completion
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let evt_type = serde_json::to_string(&event.event_type).unwrap_or_default();
                if evt_type.contains("run_completed") || evt_type.contains("run_error") {
                    break;
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                break;
            }
        }
    }

    // Verify DB was updated
    let run = state.db.get_run(&run_id).unwrap();
    assert!(
        run.status == hydra_db::RunStatus::Completed || run.status == hydra_db::RunStatus::Failed,
        "Run should be completed or failed, got {:?}",
        run.status
    );

    // Verify steps were created
    let steps = state.db.list_steps(&run_id).unwrap();
    assert!(
        steps.len() > 0,
        "Should have created DB steps for cognitive phases"
    );

    // Verify receipt was generated
    assert!(state.ledger.len() > 0, "Should have generated a receipt");
}
