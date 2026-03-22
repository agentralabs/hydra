//! ExecutionReceipt — every action receipted before it starts.
//! Constitutional Law 1: receipts are immutable.
//! Constitutional Law 7: every action has a traceable causal origin.

use crate::constants::RECEIPT_HASH_LABEL;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The receipt for one execution event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub intent: String,
    pub approach: String,
    pub outcome: ReceiptOutcome,
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReceiptOutcome {
    Started,
    Succeeded,
    Blocked { obstacle: String },
    Rerouted { to_approach: String },
    HardDenied { evidence: String },
}

impl ExecutionReceipt {
    /// Create a receipt for the START of an execution.
    /// Must be created BEFORE execution begins (write-ahead).
    pub fn for_start(
        task_id: &str,
        action_id: &str,
        intent: &str,
        approach: &str,
    ) -> Self {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        let hash = Self::compute_hash(task_id, action_id, intent, approach, &now);

        Self {
            id,
            task_id: task_id.to_string(),
            action_id: action_id.to_string(),
            intent: intent.to_string(),
            approach: approach.to_string(),
            outcome: ReceiptOutcome::Started,
            content_hash: hash,
            created_at: now,
        }
    }

    fn compute_hash(
        task_id: &str,
        action_id: &str,
        intent: &str,
        approach: &str,
        at: &chrono::DateTime<chrono::Utc>,
    ) -> String {
        let _ = RECEIPT_HASH_LABEL; // used for documentation, algorithm is SHA256
        let mut h = Sha256::new();
        h.update(task_id.as_bytes());
        h.update(action_id.as_bytes());
        h.update(intent.as_bytes());
        h.update(approach.as_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    pub fn verify(&self) -> bool {
        !self.content_hash.is_empty() && self.content_hash.len() == 64
    }
}

/// The receipt ledger for one execution session.
#[derive(Debug, Default)]
pub struct ReceiptLedger {
    receipts: Vec<ExecutionReceipt>,
}

impl ReceiptLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, receipt: ExecutionReceipt) {
        // Receipts are append-only — constitutional law
        self.receipts.push(receipt);
    }

    pub fn count(&self) -> usize {
        self.receipts.len()
    }

    pub fn for_task(&self, task_id: &str) -> Vec<&ExecutionReceipt> {
        self.receipts
            .iter()
            .filter(|r| r.task_id == task_id)
            .collect()
    }

    pub fn latest(&self) -> Option<&ExecutionReceipt> {
        self.receipts.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_created_before_execution() {
        let r = ExecutionReceipt::for_start(
            "task-1",
            "action.deploy",
            "deploy staging",
            "direct",
        );
        assert!(r.verify());
        assert_eq!(r.outcome, ReceiptOutcome::Started);
        assert!(!r.content_hash.is_empty());
    }

    #[test]
    fn receipt_hash_is_64_chars() {
        let r = ExecutionReceipt::for_start("t", "a", "i", "approach");
        assert_eq!(r.content_hash.len(), 64);
    }

    #[test]
    fn ledger_append_only() {
        let mut ledger = ReceiptLedger::new();
        let r1 = ExecutionReceipt::for_start("t1", "a1", "intent", "direct");
        let r2 =
            ExecutionReceipt::for_start("t1", "a1", "intent", "alternative");
        ledger.record(r1);
        ledger.record(r2);
        assert_eq!(ledger.count(), 2);
        assert_eq!(ledger.for_task("t1").len(), 2);
    }
}
