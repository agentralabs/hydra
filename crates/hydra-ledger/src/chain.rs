use crate::receipt::LedgerReceipt;

/// Result of chain verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainStatus {
    Valid,
    BrokenLink { at_sequence: u64 },
    HashMismatch { at_sequence: u64 },
    Tampered { at_sequence: u64 },
    Forked { at_sequence: u64 },
    FutureTimestamp { at_sequence: u64 },
    Empty,
}

impl ChainStatus {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid | Self::Empty)
    }

    pub fn corruption_detected(&self) -> bool {
        matches!(self, Self::HashMismatch { .. } | Self::Tampered { .. })
    }
}

/// Full chain verification result
#[derive(Debug, Clone)]
pub struct ChainVerification {
    pub status: ChainStatus,
    pub total_receipts: usize,
    pub verified_receipts: usize,
}

impl ChainVerification {
    pub fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    pub fn corruption_detected(&self) -> bool {
        self.status.corruption_detected()
    }
}

/// Verify an ordered list of receipts
pub fn verify_chain(receipts: &[LedgerReceipt]) -> ChainVerification {
    if receipts.is_empty() {
        return ChainVerification {
            status: ChainStatus::Empty,
            total_receipts: 0,
            verified_receipts: 0,
        };
    }

    let mut verified = 0;

    for (i, receipt) in receipts.iter().enumerate() {
        // Verify content hash
        if !receipt.verify_hash() {
            return ChainVerification {
                status: ChainStatus::HashMismatch {
                    at_sequence: receipt.sequence,
                },
                total_receipts: receipts.len(),
                verified_receipts: verified,
            };
        }

        // Verify chain link
        let parent = if i > 0 { Some(&receipts[i - 1]) } else { None };
        if !receipt.verify_chain_link(parent) {
            return ChainVerification {
                status: ChainStatus::BrokenLink {
                    at_sequence: receipt.sequence,
                },
                total_receipts: receipts.len(),
                verified_receipts: verified,
            };
        }

        // Check for future timestamps
        if receipt.has_future_timestamp() {
            return ChainVerification {
                status: ChainStatus::FutureTimestamp {
                    at_sequence: receipt.sequence,
                },
                total_receipts: receipts.len(),
                verified_receipts: verified,
            };
        }

        verified += 1;
    }

    ChainVerification {
        status: ChainStatus::Valid,
        total_receipts: receipts.len(),
        verified_receipts: verified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::LedgerReceiptType;

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
    fn empty_chain_is_valid() {
        let v = verify_chain(&[]);
        assert!(v.is_valid());
        assert_eq!(v.status, ChainStatus::Empty);
        assert_eq!(v.total_receipts, 0);
    }

    #[test]
    fn single_receipt_chain_valid() {
        let chain = build_chain(1);
        let v = verify_chain(&chain);
        assert!(v.is_valid());
        assert_eq!(v.verified_receipts, 1);
    }

    #[test]
    fn multi_receipt_chain_valid() {
        let chain = build_chain(5);
        let v = verify_chain(&chain);
        assert!(v.is_valid());
        assert_eq!(v.verified_receipts, 5);
        assert_eq!(v.total_receipts, 5);
    }

    #[test]
    fn tampered_hash_detected() {
        let mut chain = build_chain(3);
        chain[1].content_hash = "tampered".to_string();
        let v = verify_chain(&chain);
        assert!(!v.is_valid());
        assert!(v.corruption_detected());
        assert_eq!(v.status, ChainStatus::HashMismatch { at_sequence: 1 });
    }

    #[test]
    fn broken_link_detected() {
        let mut chain = build_chain(3);
        // Tamper with the action of receipt 1, then recompute its hash
        // but don't update receipt 2's previous_hash → broken link
        chain[1].action = "modified".to_string();
        chain[1].content_hash = LedgerReceipt::compute_hash(
            &chain[1].id,
            chain[1].sequence,
            &chain[1].action,
            &chain[1].result,
            &chain[1].timestamp,
            &chain[1].previous_hash,
        );
        let v = verify_chain(&chain);
        assert!(!v.is_valid());
        // Receipt 2 now has a broken link because its previous_hash doesn't match receipt 1's new hash
        assert_eq!(v.status, ChainStatus::BrokenLink { at_sequence: 2 });
    }

    #[test]
    fn chain_status_corruption_methods() {
        assert!(!ChainStatus::Valid.corruption_detected());
        assert!(!ChainStatus::Empty.corruption_detected());
        assert!(ChainStatus::HashMismatch { at_sequence: 0 }.corruption_detected());
        assert!(ChainStatus::Tampered { at_sequence: 0 }.corruption_detected());
        assert!(!ChainStatus::BrokenLink { at_sequence: 0 }.corruption_detected());
    }
}
