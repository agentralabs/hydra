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
