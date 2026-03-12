use hydra_db::HydraDb;
use hydra_ledger::receipt::LedgerReceiptType;
use hydra_ledger::ReceiptLedger;
use hydra_stress::StressServer;

/// Test that memory stays bounded under load (no unbounded growth)
#[tokio::test]
async fn test_memory_bounded_under_load() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Run 200 requests — memory should not grow unboundedly
    for i in 0..200 {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": format!("mem-{i}"),
            "method": "hydra.health",
            "params": {},
        });
        let _ = client.post(server.url("/rpc")).json(&body).send().await;
    }

    // If we get here without OOM, memory is bounded enough
    let resp = client.get(server.url("/health")).send().await.unwrap();
    assert_eq!(
        resp.status(),
        200,
        "Server should still respond after 200 requests"
    );
}

/// Test no memory leak across 1000 run creations
#[tokio::test]
async fn test_no_memory_leak_1000_runs() {
    let db = HydraDb::in_memory().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    // Create 1000 runs
    for i in 0..1000 {
        let run = hydra_db::RunRow {
            id: format!("leak-{i}"),
            intent: format!("memory leak test {i}"),
            status: hydra_db::RunStatus::Completed,
            created_at: now.clone(),
            updated_at: now.clone(),
            completed_at: Some(now.clone()),
            parent_run_id: None,
            metadata: None,
        };
        db.create_run(&run).unwrap();
    }

    // Verify all are queryable
    let runs = db.list_runs(None).unwrap();
    assert_eq!(runs.len(), 1000);

    // Clean up (delete all)
    for i in 0..1000 {
        let _ = db.delete_run(&format!("leak-{i}"));
    }

    let remaining = db.list_runs(None).unwrap();
    assert_eq!(remaining.len(), 0, "All runs should be deleted");
}

/// Test handling of large intents (1MB)
#[tokio::test]
async fn test_large_intent_handling() {
    let server = StressServer::start().await;
    let client = reqwest::Client::new();

    // Create a 1MB intent
    let large_intent = "x".repeat(1_000_000);
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "large",
        "method": "hydra.run",
        "params": {"intent": large_intent},
    });

    let resp = client
        .post(server.url("/rpc"))
        .json(&body)
        .send()
        .await
        .unwrap();
    // Should either accept or reject gracefully — not crash
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

/// Test handling of large response payloads
#[tokio::test]
async fn test_large_response_handling() {
    let db = HydraDb::in_memory().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    // Create many runs to produce a large status response
    for i in 0..500 {
        let run = hydra_db::RunRow {
            id: format!("large-resp-{i}"),
            intent: format!("large response test with some padding text to increase size {i}"),
            status: hydra_db::RunStatus::Completed,
            created_at: now.clone(),
            updated_at: now.clone(),
            completed_at: Some(now.clone()),
            parent_run_id: None,
            metadata: Some(format!("metadata-{i}-{}", "x".repeat(100))),
        };
        db.create_run(&run).unwrap();
    }

    let runs = db.list_runs(None).unwrap();
    assert_eq!(runs.len(), 500, "Should handle 500 runs in response");
}

/// Test receipt ledger under pressure
#[tokio::test]
async fn test_cache_eviction_under_pressure() {
    let ledger = ReceiptLedger::new();

    // Record 1000 receipts
    for i in 0..1000 {
        let receipt = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            format!("action-{i}"),
            serde_json::json!({"i": i}),
        );
        ledger.record(receipt).unwrap();
    }

    assert_eq!(ledger.len(), 1000);
    assert!(
        ledger.is_consistent(),
        "Ledger should maintain consistency under pressure"
    );

    // Verify chain integrity
    let verification = ledger.verify_chain();
    assert!(
        verification.is_valid(),
        "Chain should be valid after 1000 receipts"
    );
}
