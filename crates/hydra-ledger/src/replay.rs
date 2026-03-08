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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_chain(n: usize) -> Vec<LedgerReceipt> {
        let mut chain = Vec::new();
        let mut prev_hash = None;
        for i in 0..n {
            let r = LedgerReceipt::new(
                i as u64,
                LedgerReceiptType::ActionExecuted,
                format!("action_{}", i),
                serde_json::json!({"seq": i}),
                prev_hash,
            );
            prev_hash = Some(r.content_hash.clone());
            chain.push(r);
        }
        chain
    }

    #[test]
    fn replay_empty_chain() {
        let result = ReplayEngine::replay(&[], 0);
        assert!(result.is_empty());
        assert_eq!(result.tokens_used, 0);
        assert!(result.deterministic);
    }

    #[test]
    fn replay_full_chain() {
        let chain = build_chain(5);
        let result = ReplayEngine::replay(&chain, 0);
        assert_eq!(result.receipts.len(), 5);
    }

    #[test]
    fn replay_partial_chain() {
        let chain = build_chain(5);
        let result = ReplayEngine::replay(&chain, 2);
        assert_eq!(result.receipts.len(), 3); // sequences 2,3,4
    }

    #[test]
    fn replay_and_verify_valid_chain() {
        let chain = build_chain(3);
        let (result, valid) = ReplayEngine::replay_and_verify(&chain, 0);
        assert!(valid);
        assert_eq!(result.receipts.len(), 3);
    }

    #[test]
    fn replay_and_verify_tampered_chain() {
        let mut chain = build_chain(3);
        chain[1].action = "tampered".to_string();
        let (_result, valid) = ReplayEngine::replay_and_verify(&chain, 0);
        assert!(!valid);
    }

    #[test]
    fn generate_undo_creates_compensating_receipts() {
        let chain = build_chain(5);
        let undo = ReplayEngine::generate_undo(&chain, 2, 4);
        // Should undo sequences 3 and 4 (receipts after sequence 2, up to 4)
        assert_eq!(undo.len(), 2);
        for u in &undo {
            assert_eq!(u.receipt_type, LedgerReceiptType::UndoPerformed);
            assert!(u.action.starts_with("undo:"));
        }
    }

    #[test]
    fn generate_undo_empty_range() {
        let chain = build_chain(3);
        let undo = ReplayEngine::generate_undo(&chain, 5, 4);
        assert!(undo.is_empty());
    }

    #[test]
    fn undo_receipts_are_hash_valid() {
        let chain = build_chain(5);
        let undo = ReplayEngine::generate_undo(&chain, 2, 4);
        for u in &undo {
            assert!(u.verify_hash());
        }
    }

    #[test]
    fn undo_receipts_chain_properly() {
        let chain = build_chain(3);
        let undo = ReplayEngine::generate_undo(&chain, 0, 2);
        // Each undo should reference the previous one's hash
        assert_eq!(undo[0].previous_hash, Some(chain.last().unwrap().content_hash.clone()));
        if undo.len() > 1 {
            assert_eq!(undo[1].previous_hash, Some(undo[0].content_hash.clone()));
        }
    }
}
