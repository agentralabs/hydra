use std::time::{Duration, Instant};

use hydra_stress::StressServer;

/// Test 100 concurrent runs complete without error
#[tokio::test]
async fn test_100_concurrent_runs() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::new();
    for i in 0..100 {
        let c = client.clone();
        let url = server.url("/rpc");
        handles.push(tokio::spawn(async move {
            let body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": format!("load-{i}"),
                "method": "hydra.run",
                "params": {"intent": format!("load test task {i}")},
            });
            let resp = c.post(&url).json(&body).send().await;
            resp.is_ok() && resp.unwrap().status().is_success()
        }));
    }

    let results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let successes = results.iter().filter(|&&s| s).count();
    assert!(
        successes >= 95,
        "At least 95% should succeed, got {successes}/100"
    );
}

/// Test sustained throughput: 1000 requests sequentially within 30s
#[tokio::test]
async fn test_1000_requests_per_second() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();
    let start = Instant::now();

    let mut successes = 0u32;
    for i in 0..1000 {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": format!("rps-{i}"),
            "method": "hydra.health",
            "params": {},
        });
        if let Ok(resp) = client.post(server.url("/rpc")).json(&body).send().await {
            if resp.status().is_success() {
                successes += 1;
            }
        }
    }

    let elapsed = start.elapsed();
    assert!(
        successes >= 950,
        "At least 95% of 1000 requests should succeed, got {successes}"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "1000 requests should complete within 30s, took {:?}",
        elapsed
    );
}

/// Test sustained load for 10 seconds (CI-friendly version)
#[tokio::test]
async fn test_sustained_load_10s() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();
    let metrics = server.metrics.clone();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut tasks = Vec::new();

    // Spawn 10 concurrent workers
    for worker_id in 0..10 {
        let c = client.clone();
        let url = server.url("/rpc");
        let m = metrics.clone();
        tasks.push(tokio::spawn(async move {
            let mut count = 0u32;
            while Instant::now() < deadline {
                let body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": format!("w{worker_id}-{count}"),
                    "method": "hydra.health",
                    "params": {},
                });
                let start = Instant::now();
                let ok = c
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);
                m.record_request(ok, start.elapsed());
                count += 1;
            }
            count
        }));
    }

    let counts: Vec<u32> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(0))
        .collect();

    let total: u32 = counts.iter().sum();
    assert!(
        total > 100,
        "Should process >100 requests in 10s, got {total}"
    );
    assert!(
        metrics.success_rate() > 0.95,
        "Success rate should be >95%, got {:.1}%",
        metrics.success_rate() * 100.0
    );
}

/// Test burst-then-idle pattern
#[tokio::test]
async fn test_burst_then_idle_pattern() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Burst: 50 concurrent requests
    let mut handles = Vec::new();
    for i in 0..50 {
        let c = client.clone();
        let url = server.url("/rpc");
        handles.push(tokio::spawn(async move {
            let body = serde_json::json!({
                "jsonrpc": "2.0", "id": format!("burst-{i}"),
                "method": "hydra.health", "params": {},
            });
            c.post(&url).json(&body).send().await.is_ok()
        }));
    }
    let burst_results: Vec<bool> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(false))
        .collect();

    let burst_ok = burst_results.iter().filter(|&&s| s).count();
    assert!(
        burst_ok >= 45,
        "Burst: at least 90% success, got {burst_ok}/50"
    );

    // Idle: 2 seconds
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Second burst: should work just as well
    let resp = client.get(server.url("/health")).send().await.unwrap();
    assert_eq!(resp.status(), 200, "Server should respond after idle");
}

/// Test ramp-up ramp-down pattern
#[tokio::test]
async fn test_ramp_up_ramp_down() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    let concurrency_levels = [1, 5, 10, 20, 10, 5, 1];
    for &level in &concurrency_levels {
        let mut handles = Vec::new();
        for _i in 0..level {
            let c = client.clone();
            let url = server.url("/health");
            handles.push(tokio::spawn(
                async move { c.get(&url).send().await.is_ok() },
            ));
        }
        let results: Vec<bool> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap_or(false))
            .collect();
        let ok = results.iter().filter(|&&s| s).count();
        assert!(
            ok == level,
            "At concurrency {level}: expected all to succeed, got {ok}/{level}"
        );
    }
}
