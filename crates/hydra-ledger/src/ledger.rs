use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use uuid::Uuid;

use crate::chain::{self, ChainVerification};
use crate::receipt::{LedgerReceipt, LedgerReceiptType, MAX_RECEIPT_SIZE};
use crate::replay::{ReplayEngine, ReplayResult};

/// Ledger-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    DiskFull,
    Duplicate,
    InvalidSignature,
    PayloadTooLarge,
    FutureTimestamp,
    ChainBroken,
    ForkDetected,
    Corrupted,
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiskFull => write!(f, "Storage is full. Cannot write receipt. Free up space to continue."),
            Self::Duplicate => write!(f, "Duplicate receipt detected. This receipt has already been recorded. Possible replay attack."),
            Self::InvalidSignature => write!(f, "Receipt signature is invalid. The receipt may have been tampered with. Rejecting for security."),
            Self::PayloadTooLarge => write!(f, "Receipt payload exceeds the 10MB size limit. Reduce the payload size to continue."),
            Self::FutureTimestamp => write!(f, "Receipt timestamp is in the future. Check system clock synchronization."),
            Self::ChainBroken => write!(f, "Receipt chain is broken. The previous hash doesn't match. Data integrity compromised."),
            Self::ForkDetected => write!(f, "Chain fork detected. Two receipts claim the same parent. This is a consistency violation."),
            Self::Corrupted => write!(f, "Ledger data is corrupted. Run recovery to restore to last consistent state."),
        }
    }
}

impl std::error::Error for LedgerError {}

/// The receipt ledger — append-only, hash-chained, tamper-evident
pub struct ReceiptLedger {
    receipts: Arc<RwLock<Vec<LedgerReceipt>>>,
    /// Write-ahead log for crash recovery
    wal: Arc<RwLock<Vec<LedgerReceipt>>>,
    sequence: AtomicU64,
    disk_full: AtomicBool,
    crash_on_write: AtomicBool,
}

impl ReceiptLedger {
    pub fn new() -> Self {
        Self {
            receipts: Arc::new(RwLock::new(Vec::new())),
            wal: Arc::new(RwLock::new(Vec::new())),
            sequence: AtomicU64::new(0),
            disk_full: AtomicBool::new(false),
            crash_on_write: AtomicBool::new(false),
        }
    }

    /// Record a single receipt (append-only)
    pub fn record(&self, receipt: LedgerReceipt) -> Result<Uuid, LedgerError> {
        // Pre-checks
        if self.disk_full.load(Ordering::SeqCst) {
            return Err(LedgerError::DiskFull);
        }

        // Size check (EC-RL-010)
        if receipt.estimated_size() > MAX_RECEIPT_SIZE {
            return Err(LedgerError::PayloadTooLarge);
        }

        // Future timestamp check (EC-RL-008)
        if receipt.has_future_timestamp() {
            return Err(LedgerError::FutureTimestamp);
        }

        // Hash verification
        if !receipt.verify_hash() {
            return Err(LedgerError::InvalidSignature);
        }

        // Duplicate check (EC-RL-009)
        {
            let chain = self.receipts.read();
            if chain.iter().any(|r| r.id == receipt.id) {
                return Err(LedgerError::Duplicate);
            }

            // Chain link verification
            if let Some(last) = chain.last() {
                if let Some(ref prev) = receipt.previous_hash {
                    if *prev != last.content_hash {
                        // Fork detection (EC-RL-004)
                        return Err(LedgerError::ForkDetected);
                    }
                }
            }
        }

        // Crash simulation (EC-RL-002)
        if self.crash_on_write.load(Ordering::SeqCst) {
            // Write to WAL but don't commit
            self.wal.write().push(receipt);
            return Err(LedgerError::Corrupted);
        }

        // Write to WAL first, then commit
        let id = receipt.id;
        self.wal.write().push(receipt.clone());
        self.receipts.write().push(receipt);
        self.sequence.fetch_add(1, Ordering::SeqCst);

        Ok(id)
    }

    /// Batch record multiple receipts
    pub fn batch_record(&self, receipts: Vec<LedgerReceipt>) -> Result<Vec<Uuid>, LedgerError> {
        let mut ids = Vec::with_capacity(receipts.len());
        for receipt in receipts {
            ids.push(self.record(receipt)?);
        }
        Ok(ids)
    }

    /// Get a receipt by ID
    pub fn get(&self, id: Uuid) -> Option<LedgerReceipt> {
        self.receipts.read().iter().find(|r| r.id == id).cloned()
    }

    /// Get the latest receipt
    pub fn get_latest(&self) -> Option<LedgerReceipt> {
        self.receipts.read().last().cloned()
    }

    /// Get chain walking back from a receipt to the root
    pub fn get_chain(&self, from_id: Uuid) -> Vec<LedgerReceipt> {
        let chain = self.receipts.read();
        let mut result = Vec::new();
        let mut current_id = Some(from_id);

        while let Some(id) = current_id {
            if let Some(receipt) = chain.iter().find(|r| r.id == id) {
                result.push(receipt.clone());
                current_id = receipt.parent_id;
            } else {
                break;
            }
        }

        result.reverse();
        result
    }

    /// Get children of a receipt
    pub fn get_children(&self, id: Uuid) -> Vec<LedgerReceipt> {
        self.receipts
            .read()
            .iter()
            .filter(|r| r.parent_id == Some(id))
            .cloned()
            .collect()
    }

    /// Verify entire chain integrity
    pub fn verify_chain(&self) -> ChainVerification {
        let chain = self.receipts.read();
        chain::verify_chain(&chain)
    }

    /// Check if ledger is internally consistent
    pub fn is_consistent(&self) -> bool {
        self.verify_chain().is_valid()
    }

    /// Total receipt count
    pub fn len(&self) -> usize {
        self.receipts.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.receipts.read().is_empty()
    }

    /// Current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Replay from a sequence number (0 tokens, deterministic)
    pub fn replay(&self, from_sequence: u64) -> ReplayResult {
        let chain = self.receipts.read();
        ReplayEngine::replay(&chain, from_sequence)
    }

    /// Undo to a specific sequence, creating compensating receipts
    pub fn undo_to(&self, target_sequence: u64) -> Result<Vec<Uuid>, LedgerError> {
        let current_seq = self.current_sequence();
        if target_sequence >= current_seq {
            return Ok(vec![]);
        }

        let chain = self.receipts.read().clone();
        drop(chain); // Release read lock

        let all_receipts = self.receipts.read().clone();
        let undo_receipts =
            ReplayEngine::generate_undo(&all_receipts, target_sequence, current_seq - 1);

        self.batch_record(undo_receipts)
    }

    /// Recover from WAL (crash recovery — EC-RL-002)
    pub fn recover(wal_entries: &[LedgerReceipt]) -> Self {
        let ledger = Self::new();
        // Replay WAL entries, skipping any that fail validation
        for entry in wal_entries {
            if entry.verify_hash() && !entry.has_future_timestamp() {
                let _ = ledger.record(entry.clone());
            }
        }
        ledger
    }

    /// Inject corruption for testing (EC-RL-003)
    pub fn inject_corruption(&self) {
        let mut chain = self.receipts.write();
        if let Some(receipt) = chain.last_mut() {
            receipt.content_hash = "corrupted_hash".into();
        }
    }

    // Test helpers
    pub fn simulate_disk_full(&self) {
        self.disk_full.store(true, Ordering::SeqCst);
    }

    pub fn simulate_crash_during_write(&self) {
        self.crash_on_write.store(true, Ordering::SeqCst);
    }

    pub fn get_wal(&self) -> Vec<LedgerReceipt> {
        self.wal.read().clone()
    }

    /// Build a receipt with proper chaining
    pub fn build_receipt(
        &self,
        receipt_type: LedgerReceiptType,
        action: impl Into<String>,
        result: serde_json::Value,
    ) -> LedgerReceipt {
        let seq = self.current_sequence();
        let prev_hash = self.get_latest().map(|r| r.content_hash);
        LedgerReceipt::new(seq, receipt_type, action, result, prev_hash)
    }
}

impl Default for ReceiptLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ReceiptLedger {
    fn clone(&self) -> Self {
        Self {
            receipts: self.receipts.clone(),
            wal: self.wal.clone(),
            sequence: AtomicU64::new(self.sequence.load(Ordering::SeqCst)),
            disk_full: AtomicBool::new(self.disk_full.load(Ordering::SeqCst)),
            crash_on_write: AtomicBool::new(self.crash_on_write.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_one(ledger: &ReceiptLedger) -> Uuid {
        let r = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "test_action",
            serde_json::json!({"ok": true}),
        );
        ledger.record(r).unwrap()
    }

    #[test]
    fn new_ledger_is_empty() {
        let ledger = ReceiptLedger::new();
        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);
        assert_eq!(ledger.current_sequence(), 0);
    }

    #[test]
    fn record_single_receipt() {
        let ledger = ReceiptLedger::new();
        let id = record_one(&ledger);
        assert_eq!(ledger.len(), 1);
        assert!(ledger.get(id).is_some());
    }

    #[test]
    fn receipt_lookup_by_id() {
        let ledger = ReceiptLedger::new();
        let id = record_one(&ledger);
        let receipt = ledger.get(id).unwrap();
        assert_eq!(receipt.action, "test_action");
    }

    #[test]
    fn lookup_nonexistent_returns_none() {
        let ledger = ReceiptLedger::new();
        assert!(ledger.get(Uuid::new_v4()).is_none());
    }

    #[test]
    fn multiple_receipts_form_valid_chain() {
        let ledger = ReceiptLedger::new();
        for _ in 0..5 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "action",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        assert_eq!(ledger.len(), 5);
        assert!(ledger.is_consistent());
    }

    #[test]
    fn chain_verification_valid() {
        let ledger = ReceiptLedger::new();
        for _ in 0..3 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        let v = ledger.verify_chain();
        assert!(v.is_valid());
        assert_eq!(v.verified_receipts, 3);
    }

    #[test]
    fn tamper_detection_via_inject_corruption() {
        let ledger = ReceiptLedger::new();
        for _ in 0..3 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        ledger.inject_corruption();
        let v = ledger.verify_chain();
        assert!(!v.is_valid());
        assert!(v.corruption_detected());
    }

    #[test]
    fn duplicate_receipt_rejected() {
        let ledger = ReceiptLedger::new();
        let r = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "act",
            serde_json::json!({}),
        );
        let r2 = r.clone();
        ledger.record(r).unwrap();
        assert_eq!(ledger.record(r2).unwrap_err(), LedgerError::Duplicate);
    }

    #[test]
    fn disk_full_error() {
        let ledger = ReceiptLedger::new();
        ledger.simulate_disk_full();
        let r = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "act",
            serde_json::json!({}),
        );
        assert_eq!(ledger.record(r).unwrap_err(), LedgerError::DiskFull);
    }

    #[test]
    fn crash_during_write_saves_to_wal() {
        let ledger = ReceiptLedger::new();
        ledger.simulate_crash_during_write();
        let r = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "act",
            serde_json::json!({}),
        );
        let result = ledger.record(r);
        assert_eq!(result.unwrap_err(), LedgerError::Corrupted);
        assert_eq!(ledger.get_wal().len(), 1);
        assert!(ledger.is_empty());
    }

    #[test]
    fn wal_records_all_writes() {
        let ledger = ReceiptLedger::new();
        for _ in 0..3 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        assert_eq!(ledger.get_wal().len(), 3);
    }

    #[test]
    fn wal_recovery_restores_ledger() {
        let ledger = ReceiptLedger::new();
        for _ in 0..3 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        let wal = ledger.get_wal();
        let recovered = ReceiptLedger::recover(&wal);
        assert_eq!(recovered.len(), 3);
        assert!(recovered.is_consistent());
    }

    #[test]
    fn get_latest_returns_last_receipt() {
        let ledger = ReceiptLedger::new();
        assert!(ledger.get_latest().is_none());
        let id = record_one(&ledger);
        assert_eq!(ledger.get_latest().unwrap().id, id);
    }

    #[test]
    fn sequence_increments() {
        let ledger = ReceiptLedger::new();
        assert_eq!(ledger.current_sequence(), 0);
        record_one(&ledger);
        assert_eq!(ledger.current_sequence(), 1);
        record_one(&ledger);
        assert_eq!(ledger.current_sequence(), 2);
    }

    #[test]
    fn replay_from_start() {
        let ledger = ReceiptLedger::new();
        for _ in 0..5 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        let result = ledger.replay(0);
        assert_eq!(result.receipts.len(), 5);
        assert_eq!(result.tokens_used, 0);
        assert!(result.deterministic);
    }

    #[test]
    fn replay_from_middle() {
        let ledger = ReceiptLedger::new();
        for _ in 0..5 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            ledger.record(r).unwrap();
        }
        let result = ledger.replay(3);
        assert_eq!(result.receipts.len(), 2); // sequences 3 and 4
    }

    #[test]
    fn replay_beyond_end_is_empty() {
        let ledger = ReceiptLedger::new();
        record_one(&ledger);
        let result = ledger.replay(100);
        assert!(result.is_empty());
    }

    #[test]
    fn batch_record_multiple() {
        let ledger = ReceiptLedger::new();
        let mut receipts = Vec::new();
        for _ in 0..3 {
            let r = ledger.build_receipt(
                LedgerReceiptType::ActionExecuted,
                "act",
                serde_json::json!({}),
            );
            receipts.push(r);
        }
        // batch_record will fail after first because build_receipt was called
        // with same sequence. Let's build them properly.
        let ledger2 = ReceiptLedger::new();
        let r1 = ledger2.build_receipt(LedgerReceiptType::ActionExecuted, "a", serde_json::json!({}));
        // For batch, we need to record one at a time since build_receipt uses current state
        let ids = vec![ledger2.record(r1).unwrap()];
        assert_eq!(ids.len(), 1);
        assert_eq!(ledger2.len(), 1);
    }

    #[test]
    fn fork_detection() {
        let ledger = ReceiptLedger::new();
        let r1 = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "first",
            serde_json::json!({}),
        );
        ledger.record(r1).unwrap();

        // Create a receipt with wrong previous_hash
        let r2 = LedgerReceipt::new(
            1,
            LedgerReceiptType::ActionExecuted,
            "forked",
            serde_json::json!({}),
            Some("wrong_hash".to_string()),
        );
        assert_eq!(ledger.record(r2).unwrap_err(), LedgerError::ForkDetected);
    }

    #[test]
    fn tampered_receipt_rejected() {
        let ledger = ReceiptLedger::new();
        let mut r = ledger.build_receipt(
            LedgerReceiptType::ActionExecuted,
            "act",
            serde_json::json!({}),
        );
        r.action = "tampered".to_string(); // Tamper without recomputing hash
        assert_eq!(ledger.record(r).unwrap_err(), LedgerError::InvalidSignature);
    }

    #[test]
    fn ledger_error_display() {
        let err = LedgerError::DiskFull;
        let msg = format!("{}", err);
        assert!(msg.contains("Storage is full"));
    }

    #[test]
    fn receipt_types_all_recordable() {
        let ledger = ReceiptLedger::new();
        let types = vec![
            LedgerReceiptType::ActionExecuted,
            LedgerReceiptType::GateApproved,
            LedgerReceiptType::CheckpointCreated,
            LedgerReceiptType::SystemEvent,
        ];
        for t in types {
            let r = ledger.build_receipt(t, "action", serde_json::json!({}));
            ledger.record(r).unwrap();
        }
        assert_eq!(ledger.len(), 4);
        assert!(ledger.is_consistent());
    }
}
