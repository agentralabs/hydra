use std::sync::Arc;
use std::time::{Duration, Instant};

use hydra_ledger::ledger::{LedgerError, ReceiptLedger};
use hydra_ledger::receipt::{LedgerReceipt, LedgerReceiptType};

fn build_and_record(ledger: &ReceiptLedger, action: &str) -> uuid::Uuid {
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        action,
        serde_json::json!({"status": "ok"}),
    );
    ledger.record(receipt).unwrap()
}

// ═══════════════════════════════════════════════════════════
// EDGE CASES (EC-RL-001 through EC-RL-010)
// ═══════════════════════════════════════════════════════════

/// EC-RL-001: Disk full during write
#[test]
fn test_ec_rl_001_disk_full() {
    let ledger = ReceiptLedger::new();
    build_and_record(&ledger, "before_full");
    assert!(ledger.is_consistent());

    ledger.simulate_disk_full();
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "fail",
        serde_json::json!({}),
    );
    let result = ledger.record(receipt);
    assert_eq!(result.unwrap_err(), LedgerError::DiskFull);
    // Data should still be consistent
    assert!(ledger.is_consistent());
}

/// EC-RL-002: Crash recovery via WAL
#[test]
fn test_ec_rl_002_crash_recovery() {
    let ledger = ReceiptLedger::new();
    build_and_record(&ledger, "committed_1");
    build_and_record(&ledger, "committed_2");

    // Simulate crash during next write
    ledger.simulate_crash_during_write();
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "crash",
        serde_json::json!({}),
    );
    let _ = ledger.record(receipt); // This "crashes"

    // Recover from WAL
    let wal = ledger.get_wal();
    let recovered = ReceiptLedger::recover(&wal);
    assert!(recovered.is_consistent());
    assert!(recovered.len() >= 2); // At least the committed receipts
}

/// EC-RL-003: Corrupted data detection
#[test]
fn test_ec_rl_003_corrupted_data() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    ledger.inject_corruption();
    let verification = ledger.verify_chain();
    assert!(verification.corruption_detected());
}

/// EC-RL-004: Chain fork detection
#[test]
fn test_ec_rl_004_chain_fork() {
    let ledger = ReceiptLedger::new();
    build_and_record(&ledger, "root");
    let latest = ledger.get_latest().unwrap();

    // Try to create a fork — a second receipt claiming the same parent
    build_and_record(&ledger, "legit_child");

    // Now try to record a receipt with the OLD parent hash (fork attempt)
    let fork = LedgerReceipt::new(
        2,
        LedgerReceiptType::ActionExecuted,
        "forked_child",
        serde_json::json!({}),
        Some(latest.content_hash.clone()), // Points to root, not to legit_child
    );
    let result = ledger.record(fork);
    assert_eq!(result.unwrap_err(), LedgerError::ForkDetected);
}

/// EC-RL-005: Large history (10K+ receipts) — performance
#[test]
fn test_ec_rl_005_large_history() {
    let ledger = ReceiptLedger::new();
    for i in 0..10_000 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    assert_eq!(ledger.len(), 10_000);

    // Lookup first receipt by scanning — should be fast
    let start = Instant::now();
    let latest = ledger.get_latest();
    assert!(latest.is_some());
    assert!(start.elapsed() < Duration::from_secs(1));
}

/// EC-RL-006: Concurrent reads and writes
#[tokio::test]
async fn test_ec_rl_006_concurrent_access() {
    let ledger = Arc::new(ReceiptLedger::new());

    let writer = ledger.clone();
    let reader = ledger.clone();

    let write_task = tokio::spawn(async move {
        for i in 0..100 {
            let receipt = writer.build_receipt(
                LedgerReceiptType::ActionExecuted,
                format!("write_{i}"),
                serde_json::json!({"i": i}),
            );
            let _ = writer.record(receipt);
        }
    });

    let read_task = tokio::spawn(async move {
        for _ in 0..100 {
            let _ = reader.get_latest();
            let _ = reader.len();
        }
    });

    // Neither should deadlock or panic
    tokio::try_join!(write_task, read_task).unwrap();
    assert!(ledger.len() > 0);
}

/// EC-RL-007: Invalid signature/hash
#[test]
fn test_ec_rl_007_invalid_signature() {
    let ledger = ReceiptLedger::new();
    let mut receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "tampered",
        serde_json::json!({}),
    );
    receipt.content_hash = "tampered_hash_value".into();
    let result = ledger.record(receipt);
    assert_eq!(result.unwrap_err(), LedgerError::InvalidSignature);
}

/// EC-RL-008: Future timestamp
#[test]
fn test_ec_rl_008_future_timestamp() {
    let ledger = ReceiptLedger::new();
    let mut receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "future",
        serde_json::json!({}),
    );
    receipt.timestamp = chrono::Utc::now() + chrono::Duration::days(1);
    // Recompute hash with new timestamp
    receipt.content_hash = LedgerReceipt::compute_hash(
        &receipt.id,
        receipt.sequence,
        &receipt.action,
        &receipt.result,
        &receipt.timestamp,
        &receipt.previous_hash,
    );
    let result = ledger.record(receipt);
    assert_eq!(result.unwrap_err(), LedgerError::FutureTimestamp);
}

/// EC-RL-009: Replay attack (duplicate receipt)
#[test]
fn test_ec_rl_009_replay_attack() {
    let ledger = ReceiptLedger::new();
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "original",
        serde_json::json!({}),
    );
    let receipt_clone = receipt.clone();
    ledger.record(receipt).unwrap();
    let result = ledger.record(receipt_clone);
    assert_eq!(result.unwrap_err(), LedgerError::Duplicate);
}

/// EC-RL-010: Oversized receipt
#[test]
fn test_ec_rl_010_oversized_receipt() {
    let ledger = ReceiptLedger::new();
    let huge_data = "x".repeat(11 * 1024 * 1024); // 11MB > 10MB limit
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "oversized",
        serde_json::json!({"data": huge_data}),
    );
    let result = ledger.record(receipt);
    assert_eq!(result.unwrap_err(), LedgerError::PayloadTooLarge);
}

// ═══════════════════════════════════════════════════════════
// RECEIPT SERIALIZATION
// ═══════════════════════════════════════════════════════════

#[test]
fn test_receipt_serde_roundtrip() {
    let receipt = LedgerReceipt::new(
        0,
        LedgerReceiptType::ActionExecuted,
        "test",
        serde_json::json!({"key": "value"}),
        None,
    );
    let json = serde_json::to_string(&receipt).unwrap();
    let deserialized: LedgerReceipt = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, receipt.id);
    assert_eq!(deserialized.content_hash, receipt.content_hash);
}
