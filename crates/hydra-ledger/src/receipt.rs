use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum receipt payload size (10MB)
pub const MAX_RECEIPT_SIZE: usize = 10 * 1024 * 1024;

/// A ledger receipt — immutable, hash-chained, signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerReceipt {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub sequence: u64,
    pub receipt_type: LedgerReceiptType,
    pub action: String,
    pub result: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub content_hash: String,
    pub previous_hash: Option<String>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerReceiptType {
    ActionExecuted,
    ActionFailed,
    ActionRolledBack,
    GateApproved,
    GateDenied,
    GateBlocked,
    CheckpointCreated,
    UndoPerformed,
    SystemEvent,
}

impl LedgerReceipt {
    pub fn new(
        sequence: u64,
        receipt_type: LedgerReceiptType,
        action: impl Into<String>,
        result: serde_json::Value,
        previous_hash: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let action = action.into();
        let timestamp = Utc::now();
        let content_hash =
            Self::compute_hash(&id, sequence, &action, &result, &timestamp, &previous_hash);
        Self {
            id,
            parent_id: None,
            sequence,
            receipt_type,
            action,
            result,
            timestamp,
            content_hash,
            previous_hash,
            signature: String::new(), // Filled by signer
        }
    }

    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Compute content hash for tamper detection
    pub fn compute_hash(
        id: &Uuid,
        sequence: u64,
        action: &str,
        result: &serde_json::Value,
        timestamp: &DateTime<Utc>,
        previous_hash: &Option<String>,
    ) -> String {
        let input = format!(
            "{}|{}|{}|{}|{}|{}",
            id,
            sequence,
            action,
            result,
            timestamp.to_rfc3339(),
            previous_hash.as_deref().unwrap_or("genesis"),
        );
        format!("{:016x}", djb2_hash(&input))
    }

    /// Verify this receipt's content hash matches its data
    pub fn verify_hash(&self) -> bool {
        let expected = Self::compute_hash(
            &self.id,
            self.sequence,
            &self.action,
            &self.result,
            &self.timestamp,
            &self.previous_hash,
        );
        self.content_hash == expected
    }

    /// Verify chain link: this receipt's previous_hash matches parent's content_hash
    pub fn verify_chain_link(&self, parent: Option<&LedgerReceipt>) -> bool {
        match (&self.previous_hash, parent) {
            (None, None) => self.sequence == 0,
            (Some(prev), Some(p)) => *prev == p.content_hash && self.sequence == p.sequence + 1,
            _ => false,
        }
    }

    /// Estimated size in bytes
    pub fn estimated_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }

    /// Check if timestamp is in the future (> 60s tolerance)
    pub fn has_future_timestamp(&self) -> bool {
        self.timestamp > Utc::now() + chrono::Duration::seconds(60)
    }
}

fn djb2_hash(input: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn genesis_receipt() -> LedgerReceipt {
        LedgerReceipt::new(
            0,
            LedgerReceiptType::ActionExecuted,
            "test_action",
            serde_json::json!({"status": "ok"}),
            None,
        )
    }

    #[test]
    fn create_receipt_has_valid_hash() {
        let r = genesis_receipt();
        assert!(r.verify_hash());
    }

    #[test]
    fn receipt_sequence_and_type() {
        let r = genesis_receipt();
        assert_eq!(r.sequence, 0);
        assert_eq!(r.receipt_type, LedgerReceiptType::ActionExecuted);
    }

    #[test]
    fn genesis_receipt_has_no_previous_hash() {
        let r = genesis_receipt();
        assert!(r.previous_hash.is_none());
    }

    #[test]
    fn chained_receipt_has_previous_hash() {
        let r0 = genesis_receipt();
        let r1 = LedgerReceipt::new(
            1,
            LedgerReceiptType::ActionExecuted,
            "action_2",
            serde_json::json!({}),
            Some(r0.content_hash.clone()),
        );
        assert_eq!(r1.previous_hash, Some(r0.content_hash));
        assert!(r1.verify_hash());
    }

    #[test]
    fn verify_chain_link_genesis() {
        let r = genesis_receipt();
        assert!(r.verify_chain_link(None));
    }

    #[test]
    fn verify_chain_link_valid_pair() {
        let r0 = genesis_receipt();
        let r1 = LedgerReceipt::new(
            1,
            LedgerReceiptType::ActionExecuted,
            "next",
            serde_json::json!({}),
            Some(r0.content_hash.clone()),
        );
        assert!(r1.verify_chain_link(Some(&r0)));
    }

    #[test]
    fn verify_chain_link_invalid_sequence() {
        let r0 = genesis_receipt();
        let r1 = LedgerReceipt::new(
            5, // wrong sequence
            LedgerReceiptType::ActionExecuted,
            "next",
            serde_json::json!({}),
            Some(r0.content_hash.clone()),
        );
        assert!(!r1.verify_chain_link(Some(&r0)));
    }

    #[test]
    fn tampered_receipt_fails_hash_verification() {
        let mut r = genesis_receipt();
        r.action = "tampered_action".to_string();
        assert!(!r.verify_hash());
    }

    #[test]
    fn with_parent_sets_parent_id() {
        let parent_id = Uuid::new_v4();
        let r = genesis_receipt().with_parent(parent_id);
        assert_eq!(r.parent_id, Some(parent_id));
    }

    #[test]
    fn with_signature_sets_signature() {
        let r = genesis_receipt().with_signature("sig123");
        assert_eq!(r.signature, "sig123");
    }

    #[test]
    fn estimated_size_is_positive() {
        let r = genesis_receipt();
        assert!(r.estimated_size() > 0);
    }

    #[test]
    fn has_future_timestamp_returns_false_for_now() {
        let r = genesis_receipt();
        assert!(!r.has_future_timestamp());
    }

    #[test]
    fn djb2_deterministic() {
        let h1 = djb2_hash("hello world");
        let h2 = djb2_hash("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn djb2_different_inputs_different_hashes() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("world");
        assert_ne!(h1, h2);
    }
}
