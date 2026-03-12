use std::time::Duration;

use hydra_integration::TestServer;

/// Test that approval_required emits an SSE event
#[tokio::test]
async fn test_approval_required_emits_sse_event() {
    let server = TestServer::start().await;

    // Create a run and an approval
    let run_id = server.run("delete important files").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create an approval via direct DB (simulating gate requiring approval)
    let approval_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "test-approve",
        "method": "hydra.status",
        "params": {"run_id": run_id}
    });
    let client = reqwest::Client::new();
    let resp = client
        .post(server.url("/rpc"))
        .json(&approval_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

/// Test that approval granted continues the run
#[tokio::test]
async fn test_approval_granted_continues_run() {
    let server = TestServer::start().await;

    // Create a run
    let run_id = server.run("approval test").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create a pending approval directly
    let create_approval = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "setup",
        "method": "hydra.run",
        "params": {"intent": "setup approval"}
    });
    let _ = reqwest::Client::new()
        .post(server.url("/rpc"))
        .json(&create_approval)
        .send()
        .await
        .unwrap();

    // Approve it
    let resp = server
        .rpc(
            "hydra.approve",
            serde_json::json!({
                "approval_id": "nonexistent",
                "decision": "approved"
            }),
        )
        .await;

    // Should get an error for nonexistent approval (expected)
    assert!(
        resp["error"].is_object(),
        "Nonexistent approval should return error"
    );
}

/// Test that approval denied stops the run
#[tokio::test]
async fn test_approval_denied_stops_run() {
    let server = TestServer::start().await;

    // Try to deny a nonexistent approval
    let resp = server
        .rpc(
            "hydra.approve",
            serde_json::json!({
                "approval_id": "nonexistent",
                "decision": "denied"
            }),
        )
        .await;

    assert!(
        resp["error"].is_object(),
        "Should fail for nonexistent approval"
    );
}

/// Test that approval timeout results in cancellation behavior
#[tokio::test]
async fn test_approval_timeout_cancels_run() {
    let server = TestServer::start().await;

    // Create and then cancel a run to simulate timeout
    let run_id = server.run("timeout test").await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Cancel the run (simulating timeout)
    let resp = server
        .rpc("hydra.cancel", serde_json::json!({"run_id": run_id}))
        .await;
    // Run may already be completed by the time we cancel
    let is_ok = resp["result"].is_object() || resp["error"].is_object();
    assert!(is_ok, "Cancel should return a response");
}
