use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hydra_db::HydraDb;
use hydra_runtime::sse::SseEvent;
use hydra_runtime::EventBus;
use hydra_sisters::bridge::SisterId;
use hydra_sisters::circuit_breaker::CircuitBreaker;
use hydra_stress::StressServer;

/// Test parallel sister calls via circuit breakers
#[tokio::test]
async fn test_parallel_sister_calls() {
    let breakers: Vec<CircuitBreaker> = SisterId::all()
        .iter()
        .map(|&id| CircuitBreaker::with_defaults(id))
        .collect();

    // Simulate parallel calls to all 14 sisters
    for cb in &breakers {
        assert!(cb.allow_call(), "All circuits should be closed initially");
        cb.record_success();
    }

    // All should still be closed
    for cb in &breakers {
        assert!(cb.allow_call());
        assert_eq!(cb.success_count(), 1);
    }
}

/// Test concurrent database writes
#[tokio::test]
async fn test_concurrent_db_writes() {
    let db = HydraDb::in_memory().unwrap();
    let db_arc = Arc::new(db);

    let mut handles = Vec::new();
    for i in 0..50 {
        let db = db_arc.clone();
        handles.push(tokio::spawn(async move {
            let now = chrono::Utc::now().to_rfc3339();
            let run = hydra_db::RunRow {
                id: format!("concurrent-{i}"),
                intent: format!("concurrent write {i}"),
                status: hydra_db::RunStatus::Pending,
                created_at: now.clone(),
                updated_at: now,
                completed_at: None,
                parent_run_id: None,
                metadata: None,
            };
            db.create_run(&run).is_ok()
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let successes = results.iter().filter(|&&s| s).count();
    assert_eq!(successes, 50, "All 50 concurrent DB writes should succeed");

    // Verify all were written
    let runs = db_arc.list_runs(None).unwrap();
    assert_eq!(runs.len(), 50);
}

/// Test concurrent SSE event broadcasts
#[tokio::test]
async fn test_concurrent_sse_broadcasts() {
    let bus = Arc::new(EventBus::new(4096));
    let received = Arc::new(AtomicU32::new(0));

    // Start a subscriber
    let mut rx = bus.subscribe();
    let recv_count = received.clone();
    let subscriber = tokio::spawn(async move {
        while let Ok(_event) = rx.recv().await {
            recv_count.fetch_add(1, Ordering::Relaxed);
        }
    });

    // Publish 100 events concurrently
    let mut publishers = Vec::new();
    for i in 0..100 {
        let b = bus.clone();
        publishers.push(tokio::spawn(async move {
            b.publish(SseEvent::new(
                hydra_runtime::sse::SseEventType::Heartbeat,
                serde_json::json!({"i": i}),
            ));
        }));
    }

    futures::future::join_all(publishers).await;

    // Give subscriber time to process
    tokio::time::sleep(Duration::from_millis(100)).await;
    subscriber.abort();

    let count = received.load(Ordering::Relaxed);
    assert!(
        count >= 90,
        "Subscriber should receive >=90 of 100 events, got {count}"
    );
    assert_eq!(bus.total_published(), 100);
}

/// Test that locks don't cause contention issues
#[tokio::test]
async fn test_lock_contention_handling() {
    let db = HydraDb::in_memory().unwrap();
    let db_arc = Arc::new(db);

    // Mix of reads and writes concurrently
    let mut handles = Vec::new();
    for i in 0..100 {
        let db = db_arc.clone();
        if i % 3 == 0 {
            // Writer
            handles.push(tokio::spawn(async move {
                let now = chrono::Utc::now().to_rfc3339();
                let run = hydra_db::RunRow {
                    id: format!("lock-{i}"),
                    intent: format!("lock test {i}"),
                    status: hydra_db::RunStatus::Pending,
                    created_at: now.clone(),
                    updated_at: now,
                    completed_at: None,
                    parent_run_id: None,
                    metadata: None,
                };
                db.create_run(&run).is_ok()
            }));
        } else {
            // Reader
            handles.push(tokio::spawn(async move { db.list_runs(None).is_ok() }));
        }
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let all_ok = results.iter().all(|&s| s);
    assert!(
        all_ok,
        "All concurrent read/write operations should succeed"
    );
}

/// Test no deadlock under load with multiple shared resources
#[tokio::test]
async fn test_no_deadlock_under_load() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Hammer multiple endpoints simultaneously
    let mut handles = Vec::new();
    for i in 0..30 {
        let c = client.clone();
        let url_health = server.url("/health");
        let url_rpc = server.url("/rpc");
        let url_events = server.url("/events");

        handles.push(tokio::spawn(async move {
            match i % 3 {
                0 => c.get(&url_health).send().await.is_ok(),
                1 => {
                    let body = serde_json::json!({
                        "jsonrpc": "2.0", "id": format!("dl-{i}"),
                        "method": "hydra.health", "params": {},
                    });
                    c.post(&url_rpc).json(&body).send().await.is_ok()
                }
                _ => c.get(&url_events).send().await.is_ok(),
            }
        }));
    }

    // Use a timeout to detect deadlocks
    let results =
        tokio::time::timeout(Duration::from_secs(10), futures::future::join_all(handles)).await;

    assert!(
        results.is_ok(),
        "Should complete without deadlock within 10s"
    );
    let results: Vec<bool> = results
        .unwrap()
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();
    let ok = results.iter().filter(|&&s| s).count();
    assert!(ok >= 25, "At least 25/30 should succeed, got {ok}");
}
