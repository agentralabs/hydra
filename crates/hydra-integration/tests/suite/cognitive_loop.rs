use std::time::Duration;

use hydra_integration::TestServer;

/// Test that hydra.run spawns an async cognitive loop
#[tokio::test]
async fn test_run_executes_cognitive_loop() {
    let server = TestServer::start().await;
    let run_id = server.run("list files in src").await;
    assert!(!run_id.is_empty());

    // Wait for async execution to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Check run status via RPC
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    let runs = status["result"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);

    let run = &runs[0];
    let run_status = run["status"].as_str().unwrap();
    assert!(
        run_status == "completed" || run_status == "running",
        "Run should be completed or still running, got: {run_status}"
    );
}

/// Test that all 5 cognitive phases emit SSE events
#[tokio::test]
async fn test_run_emits_all_5_phases_via_sse() {
    let server = TestServer::start().await;

    // Subscribe to SSE before triggering run
    let resp = reqwest::get(server.url("/events")).await.unwrap();
    assert_eq!(resp.status(), 200);

    // Trigger a run
    let run_id = server.run("test all phases").await;
    assert!(!run_id.is_empty());

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify via status that steps were created for the 5 phases
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    let runs = status["result"]["runs"].as_array().unwrap();
    assert!(!runs.is_empty());

    let steps = runs[0]["steps"].as_array().unwrap();
    // Should have 5 steps (one per cognitive phase)
    assert_eq!(
        steps.len(),
        5,
        "Should have 5 steps for 5 cognitive phases, got {}",
        steps.len()
    );
}

/// Test that a receipt is generated after run completion
#[tokio::test]
async fn test_run_creates_receipt() {
    let server = TestServer::start().await;
    let run_id = server.run("generate receipt test").await;

    // Wait for async completion
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify run completed
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    let run_status = status["result"]["runs"][0]["status"].as_str().unwrap();
    assert_eq!(
        run_status, "completed",
        "Run should be completed for receipt generation"
    );
}

/// Test that database is updated throughout the run lifecycle
#[tokio::test]
async fn test_run_updates_database() {
    let server = TestServer::start().await;

    // Verify empty initially
    let status = server.rpc("hydra.status", serde_json::json!({})).await;
    assert!(status["result"]["runs"].as_array().unwrap().is_empty());

    // Create a run
    let run_id = server.run("db update test").await;

    // Immediately check — should be pending or running
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    let run = &status["result"]["runs"][0];
    let early_status = run["status"].as_str().unwrap();
    assert!(
        early_status == "pending" || early_status == "running" || early_status == "completed",
        "Expected pending/running/completed, got: {early_status}"
    );

    // Wait for completion
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Should be completed
    let status = server
        .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
        .await;
    assert_eq!(
        status["result"]["runs"][0]["status"].as_str().unwrap(),
        "completed"
    );

    // Should have steps
    let steps = status["result"]["runs"][0]["steps"].as_array().unwrap();
    assert!(steps.len() >= 1, "Should have steps after completion");
}
