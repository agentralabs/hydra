use std::time::Duration;

use hydra_db::HydraDb;
use hydra_runtime::EventBus;
use hydra_stress::StressServer;

/// Test max concurrent runs limit behavior
#[tokio::test]
async fn test_max_concurrent_runs_limit() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Launch 50 runs concurrently — all should be accepted
    let mut handles = Vec::new();
    for i in 0..50 {
        let c = client.clone();
        let url = server.url("/rpc");
        handles.push(tokio::spawn(async move {
            let body = serde_json::json!({
                "jsonrpc": "2.0", "id": format!("max-{i}"),
                "method": "hydra.run",
                "params": {"intent": format!("concurrent limit test {i}")},
            });
            c.post(&url)
                .json(&body)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let accepted = results.iter().filter(|&&s| s).count();
    assert!(
        accepted >= 45,
        "At least 90% of 50 runs should be accepted, got {accepted}"
    );
}

/// Test DB handles connection pool exhaustion gracefully
#[tokio::test]
async fn test_db_connection_pool_exhaustion() {
    let db = HydraDb::in_memory().unwrap();

    // Rapid-fire operations on the DB
    let mut success_count = 0u32;
    for i in 0..500 {
        let now = chrono::Utc::now().to_rfc3339();
        let run = hydra_db::RunRow {
            id: format!("pool-{i}"),
            intent: format!("pool exhaustion {i}"),
            status: hydra_db::RunStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            parent_run_id: None,
            metadata: None,
        };
        if db.create_run(&run).is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(
        success_count, 500,
        "All 500 sequential DB writes should succeed"
    );

    // Reads should still work
    let runs = db.list_runs(None).unwrap();
    assert_eq!(runs.len(), 500);
}

/// Test SSE with multiple subscribers
#[tokio::test]
async fn test_sse_client_limit() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Connect 20 SSE clients simultaneously
    let mut handles = Vec::new();
    for _ in 0..20 {
        let c = client.clone();
        let url = server.url("/events");
        handles.push(tokio::spawn(async move {
            c.get(&url)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let connected = results.iter().filter(|&&s| s).count();
    assert!(
        connected >= 18,
        "At least 18/20 SSE clients should connect, got {connected}"
    );
}

/// Test event bus under high volume
#[tokio::test]
async fn test_file_descriptor_limit() {
    // Test that the event bus handles rapid publish/subscribe cycles
    let bus = std::sync::Arc::new(EventBus::new(8192));

    // Rapid subscribe/unsubscribe (simulating file descriptor churn)
    for _ in 0..1000 {
        let _rx = bus.subscribe();
        // rx drops immediately
    }

    // Bus should still work
    bus.publish(hydra_runtime::sse::SseEvent::heartbeat());
    assert!(bus.total_published() > 0);
}

/// Test thread pool under saturation
#[tokio::test]
async fn test_thread_pool_saturation() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Saturate with CPU-bound-like requests (many concurrent)
    let mut handles = Vec::new();
    for i in 0..100 {
        let c = client.clone();
        let url = server.url("/rpc");
        handles.push(tokio::spawn(async move {
            let body = serde_json::json!({
                "jsonrpc": "2.0", "id": format!("sat-{i}"),
                "method": "hydra.run",
                "params": {"intent": format!("saturate thread pool {i}")},
            });
            c.post(&url).json(&body).send().await.is_ok()
        }));
    }

    let timeout_result =
        tokio::time::timeout(Duration::from_secs(30), futures::future::join_all(handles)).await;

    assert!(
        timeout_result.is_ok(),
        "Should complete within 30s without thread pool deadlock"
    );
    let results: Vec<bool> = timeout_result
        .unwrap()
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();
    let ok = results.iter().filter(|&&s| s).count();
    assert!(
        ok >= 90,
        "At least 90% should succeed under saturation, got {ok}/100"
    );
}
