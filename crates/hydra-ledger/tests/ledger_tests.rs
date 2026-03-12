use hydra_ledger::chain::ChainStatus;
use hydra_ledger::ledger::{LedgerError, ReceiptLedger};
use hydra_ledger::receipt::{LedgerReceipt, LedgerReceiptType};
use hydra_ledger::replay::ReplayEngine;

fn build_and_record(ledger: &ReceiptLedger, action: &str) -> uuid::Uuid {
    let receipt = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        action,
        serde_json::json!({"status": "ok"}),
    );
    ledger.record(receipt).unwrap()
}

// ═══════════════════════════════════════════════════════════
// CORE LEDGER TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_record_and_get() {
    let ledger = ReceiptLedger::new();
    let id = build_and_record(&ledger, "test_action");
    let receipt = ledger.get(id).unwrap();
    assert_eq!(receipt.action, "test_action");
    assert_eq!(ledger.len(), 1);
}

#[test]
fn test_append_only_chain() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    assert_eq!(ledger.len(), 5);
    assert!(ledger.is_consistent());
}

#[test]
fn test_hash_chain_integrity() {
    let ledger = ReceiptLedger::new();
    for i in 0..10 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    let verification = ledger.verify_chain();
    assert!(verification.is_valid());
    assert_eq!(verification.total_receipts, 10);
    assert_eq!(verification.verified_receipts, 10);
}

#[test]
fn test_batch_record() {
    let ledger = ReceiptLedger::new();
    let r1 = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "a",
        serde_json::json!({}),
    );
    let mut r2 = LedgerReceipt::new(
        1,
        LedgerReceiptType::ActionExecuted,
        "b",
        serde_json::json!({}),
        Some(r1.content_hash.clone()),
    );
    r2.parent_id = Some(r1.id);
    let ids = ledger.batch_record(vec![r1, r2]).unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(ledger.len(), 2);
}

#[test]
fn test_get_latest() {
    let ledger = ReceiptLedger::new();
    assert!(ledger.get_latest().is_none());
    build_and_record(&ledger, "first");
    build_and_record(&ledger, "second");
    let latest = ledger.get_latest().unwrap();
    assert_eq!(latest.action, "second");
}

#[test]
fn test_get_chain_walk() {
    let ledger = ReceiptLedger::new();
    let id1 = build_and_record(&ledger, "root");
    let r2 = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "child",
        serde_json::json!({}),
    );
    let r2 = r2.with_parent(id1);
    let id2 = ledger.record(r2).unwrap();
    let chain = ledger.get_chain(id2);
    assert_eq!(chain.len(), 2);
}

#[test]
fn test_get_children() {
    let ledger = ReceiptLedger::new();
    let parent_id = build_and_record(&ledger, "parent");
    let child = ledger.build_receipt(
        LedgerReceiptType::ActionExecuted,
        "child",
        serde_json::json!({}),
    );
    let child = child.with_parent(parent_id);
    ledger.record(child).unwrap();
    let children = ledger.get_children(parent_id);
    assert_eq!(children.len(), 1);
}

// ═══════════════════════════════════════════════════════════
// REPLAY ENGINE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_replay_deterministic() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    let replay1 = ledger.replay(2);
    let replay2 = ledger.replay(2);
    assert_eq!(replay1.receipts.len(), replay2.receipts.len());
    assert!(replay1.deterministic);
    assert!(replay2.deterministic);
    // Same content hashes
    for (r1, r2) in replay1.receipts.iter().zip(replay2.receipts.iter()) {
        assert_eq!(r1.content_hash, r2.content_hash);
    }
}

#[test]
fn test_replay_zero_tokens() {
    let ledger = ReceiptLedger::new();
    for i in 0..10 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    let result = ledger.replay(0);
    assert_eq!(result.tokens_used, 0, "Replay must use 0 tokens");
    assert_eq!(result.receipts.len(), 10);
}

#[test]
fn test_replay_and_verify() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    // Use full chain from ledger
    let all = ledger.replay(0);
    let (result, valid) = ReplayEngine::replay_and_verify(&all.receipts, 0);
    assert!(valid);
    assert!(result.deterministic);
}

// ═══════════════════════════════════════════════════════════
// UNDO TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_undo_creates_audit_trail() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    assert_eq!(ledger.len(), 5);

    // Undo back to sequence 2 (undo actions 3 and 4)
    let undo_ids = ledger.undo_to(2).unwrap();
    assert!(!undo_ids.is_empty());
    // Undo creates new receipts (audit trail preserved)
    assert!(ledger.len() > 5);
    // Undo receipts should have UndoPerformed type
    for id in &undo_ids {
        let receipt = ledger.get(*id).unwrap();
        assert_eq!(receipt.receipt_type, LedgerReceiptType::UndoPerformed);
        assert!(receipt.action.starts_with("undo:"));
    }
    // Chain should still be valid after undo
    assert!(ledger.is_consistent());
}

// ═══════════════════════════════════════════════════════════
// CHAIN VERIFICATION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_chain_verification_detects_tampering() {
    let ledger = ReceiptLedger::new();
    for i in 0..5 {
        build_and_record(&ledger, &format!("action_{i}"));
    }
    assert!(ledger.is_consistent());

    // Inject corruption
    ledger.inject_corruption();
    let verification = ledger.verify_chain();
    assert!(verification.corruption_detected());
}

#[test]
fn test_chain_verification_empty() {
    let ledger = ReceiptLedger::new();
    let v = ledger.verify_chain();
    assert!(v.is_valid());
    assert_eq!(v.status, ChainStatus::Empty);
}

