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
