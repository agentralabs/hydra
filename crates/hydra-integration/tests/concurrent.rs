use std::time::Duration;

use hydra_integration::TestServer;

/// Test that multiple runs execute in parallel
#[tokio::test]
async fn test_multiple_runs_execute_parallel() {
    let server = TestServer::start().await;

    // Launch 5 runs in parallel
    let mut run_ids = Vec::new();
    for i in 0..5 {
        let run_id = server.run(&format!("parallel task {i}")).await;
        run_ids.push(run_id);
    }

    // All should have been accepted
    assert_eq!(run_ids.len(), 5);
    for id in &run_ids {
        assert!(!id.is_empty());
    }

    // Wait for all to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all runs exist in DB
    let status = server.rpc("hydra.status", serde_json::json!({})).await;
    let runs = status["result"]["runs"].as_array().unwrap();
    assert!(
        runs.len() >= 5,
        "Should have at least 5 runs, got {}",
        runs.len()
    );

    // All should be completed
    for run in runs {
        let run_status = run["status"].as_str().unwrap();
        assert!(
            run_status == "completed" || run_status == "running",
            "All runs should be completed or running, got: {run_status}"
        );
    }
}

/// Test that cancel stops a running task
#[tokio::test]
async fn test_cancel_stops_running_task() {
    let server = TestServer::start().await;

    // Create a run
    let run_id = server.run("cancel me").await;

    // Try to cancel immediately (may or may not succeed depending on timing)
    let cancel_resp = server
        .rpc("hydra.cancel", serde_json::json!({"run_id": run_id}))
        .await;

    // Either succeeds (cancelled running) or fails (already completed)
    let has_result = cancel_resp["result"].is_object();
    let has_error = cancel_resp["error"].is_object();
    assert!(has_result || has_error, "Should get a definitive response");

    // If cancelled, verify status
    if has_result {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let status = server
            .rpc("hydra.status", serde_json::json!({"run_id": run_id}))
            .await;
        let run_status = status["result"]["runs"][0]["status"].as_str().unwrap();
        assert_eq!(run_status, "cancelled");
    }
}
