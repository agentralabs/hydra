use crate::receipt::{LedgerReceipt, LedgerReceiptType};

/// Result of a replay operation
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub receipts: Vec<LedgerReceipt>,
    pub tokens_used: u64,
    pub deterministic: bool,
}

impl ReplayResult {
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}

/// Replay engine — deterministic, 0 tokens
pub struct ReplayEngine;

impl ReplayEngine {
    /// Replay a chain of receipts from a starting point
    /// This is deterministic and uses 0 tokens (no LLM calls)
    pub fn replay(chain: &[LedgerReceipt], from_sequence: u64) -> ReplayResult {
        let relevant: Vec<LedgerReceipt> = chain
            .iter()
            .filter(|r| r.sequence >= from_sequence)
            .cloned()
            .collect();

        ReplayResult {
            receipts: relevant,
            tokens_used: 0, // Replay never uses tokens
            deterministic: true,
        }
    }

    /// Replay and verify same output (deterministic check)
    pub fn replay_and_verify(chain: &[LedgerReceipt], from_sequence: u64) -> (ReplayResult, bool) {
        let result = Self::replay(chain, from_sequence);
        // Verify all hashes still match (deterministic)
        let all_valid = result.receipts.iter().all(|r| r.verify_hash());
        (result, all_valid)
    }

    /// Generate undo receipts for a chain segment
    pub fn generate_undo(
        chain: &[LedgerReceipt],
        undo_to_sequence: u64,
        current_sequence: u64,
    ) -> Vec<LedgerReceipt> {
        let to_undo: Vec<&LedgerReceipt> = chain
            .iter()
            .filter(|r| r.sequence > undo_to_sequence && r.sequence <= current_sequence)
            .collect();

        // Generate compensating receipts in reverse order
        let mut undo_receipts = Vec::new();
        let mut seq = current_sequence + 1;
        let mut prev_hash = chain.last().map(|r| r.content_hash.clone());

        for original in to_undo.iter().rev() {
            let undo = LedgerReceipt::new(
                seq,
                LedgerReceiptType::UndoPerformed,
                format!("undo:{}", original.action),
                serde_json::json!({
                    "undone_receipt_id": original.id,
                    "undone_sequence": original.sequence,
                    "undone_action": original.action,
                }),
                prev_hash,
            )
            .with_parent(original.id);

            prev_hash = Some(undo.content_hash.clone());
            undo_receipts.push(undo);
            seq += 1;
        }

        undo_receipts
    }
}
