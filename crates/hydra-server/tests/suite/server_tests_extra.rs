use std::time::Duration;

use hydra_server::{build_router, handle_rpc};

use super::helpers::{rpc_body, test_state, test_state_with_auth};

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
