use std::time::{Duration, Instant};

use hydra_sisters::bridge::{HealthStatus, SisterId};
use hydra_sisters::bridges;
use hydra_sisters::circuit_breaker::{CircuitBreaker, CircuitState};
use hydra_sisters::SisterBridge;
use hydra_stress::StressServer;

/// Test continuous operation for 60 seconds (CI-friendly)
#[tokio::test]
async fn test_continuous_operation_60s() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();
    let start = Instant::now();
    let mut requests = 0u64;
    let mut failures = 0u64;

    while start.elapsed() < Duration::from_secs(60) {
        let resp = client.get(server.url("/health")).send().await;
        requests += 1;
        if resp.is_err() || !resp.unwrap().status().is_success() {
            failures += 1;
        }
        // Small yield to prevent busy-loop
        tokio::task::yield_now().await;
    }

    let failure_rate = failures as f64 / requests as f64;
    assert!(
        failure_rate < 0.01,
        "Failure rate should be <1% over 60s, got {:.2}% ({failures}/{requests})",
        failure_rate * 100.0
    );
    assert!(
        requests > 100,
        "Should process >100 requests in 60s, got {requests}"
    );
}

/// Test continuous operation for 1 hour (nightly CI only)
#[tokio::test]
#[ignore] // Run with: cargo test -p hydra-stress --ignored
async fn test_continuous_operation_1_hour() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();
    let start = Instant::now();
    let mut requests = 0u64;
    let mut failures = 0u64;

    while start.elapsed() < Duration::from_secs(3600) {
        let resp = client.get(server.url("/health")).send().await;
        requests += 1;
        if resp.is_err() || !resp.unwrap().status().is_success() {
            failures += 1;
        }
        // Reduce CPU usage for long test
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let failure_rate = failures as f64 / requests as f64;
    assert!(failure_rate < 0.01, "1-hour failure rate should be <1%");
}

/// Test graceful degradation when a sister fails
#[tokio::test]
async fn test_graceful_degradation_sister_failure() {
    let bridge = bridges::memory_bridge();

    // Normal operation
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);

    // Simulate failure
    bridge.set_available(false);
    assert_eq!(bridge.health_check().await, HealthStatus::Unavailable);

    // Circuit breaker should track the failure
    let cb = CircuitBreaker::with_defaults(SisterId::Memory);
    for _ in 0..5 {
        cb.record_failure();
    }
    assert_eq!(
        cb.state(),
        CircuitState::Open,
        "Circuit should open after 5 failures"
    );

    // Restore
    bridge.set_available(true);
    assert_eq!(bridge.health_check().await, HealthStatus::Healthy);
}

/// Test server survives restart simulation under load
#[tokio::test]
async fn test_restart_under_load() {
    // Start server 1
    let server1 = StressServer::start().await;
    let client = reqwest::Client::new();

    // Send some requests
    for i in 0..10 {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": format!("pre-{i}"),
            "method": "hydra.run", "params": {"intent": format!("pre-restart {i}")},
        });
        let _ = client.post(server1.url("/rpc")).json(&body).send().await;
    }

    // "Restart" — drop server1 and start server2
    drop(server1);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let server2 = StressServer::start().await;

    // Server should be healthy immediately
    let resp = client.get(server2.url("/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // Should accept new runs
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": "post-restart",
        "method": "hydra.run", "params": {"intent": "post-restart test"},
    });
    let resp = client
        .post(server2.url("/rpc"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
}

/// Test recovery after simulated OOM (resource exhaustion)
#[tokio::test]
async fn test_recovery_after_oom_kill() {
    // Simulate: server dies, new server starts, old data is gone (in-memory)
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Create some state
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": "pre-oom",
        "method": "hydra.run", "params": {"intent": "before oom"},
    });
    let _ = client.post(server.url("/rpc")).json(&body).send().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Kill and restart
    drop(server);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let new_server = StressServer::start().await;

    // New server is clean (in-memory DB)
    let status_body = serde_json::json!({
        "jsonrpc": "2.0", "id": "post-oom",
        "method": "hydra.status", "params": {},
    });
    let resp = client
        .post(new_server.url("/rpc"))
        .json(&status_body)
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let runs = body["result"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 0, "Fresh server should have no runs");
}
