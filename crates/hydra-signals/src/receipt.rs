//! Delivery receipts — proof that a signal was delivered (or failed).
//!
//! Every signal dispatch produces a delivery receipt, which is retained
//! for the hot retention window before archival.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::RECEIPT_HOT_RETENTION_SECONDS;

/// Unique identifier for a delivery receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeliveryReceiptId(String);

impl DeliveryReceiptId {
    /// Generate a new unique receipt ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for DeliveryReceiptId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeliveryReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The outcome of a signal delivery attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// Signal was delivered successfully.
    Delivered,
    /// Signal delivery failed.
    Failed {
        /// The reason for failure.
        reason: String,
    },
    /// Signal was rejected at the gate.
    Rejected {
        /// The reason for rejection.
        reason: String,
    },
    /// Signal was dropped (e.g. orphan, below weight floor).
    Dropped {
        /// The reason for dropping.
        reason: String,
    },
}

/// A delivery receipt for a single signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    /// Unique ID of this receipt.
    pub id: DeliveryReceiptId,
    /// The signal ID this receipt is for.
    pub signal_id: String,
    /// The outcome of the delivery attempt.
    pub outcome: DeliveryOutcome,
    /// When the delivery was attempted.
    pub timestamp: DateTime<Utc>,
    /// The handler that processed (or failed to process) the signal.
    pub handler: String,
}

impl DeliveryReceipt {
    /// Create a new delivery receipt.
    pub fn new(signal_id: &str, outcome: DeliveryOutcome, handler: &str) -> Self {
        Self {
            id: DeliveryReceiptId::new(),
            signal_id: signal_id.to_string(),
            outcome,
            timestamp: Utc::now(),
            handler: handler.to_string(),
        }
    }

    /// Returns true if the delivery was successful.
    pub fn is_success(&self) -> bool {
        matches!(self.outcome, DeliveryOutcome::Delivered)
    }

    /// Returns true if this receipt is still within the hot retention window.
    pub fn is_hot(&self) -> bool {
        let age = Utc::now()
            .signed_duration_since(self.timestamp)
            .num_seconds();
        age >= 0 && (age as u64) < RECEIPT_HOT_RETENTION_SECONDS
    }
}

/// Log of delivery receipts with automatic rotation.
pub struct DeliveryReceiptLog {
    /// All receipts in the hot window.
    receipts: Vec<DeliveryReceipt>,
}

impl DeliveryReceiptLog {
    /// Create a new empty receipt log.
    pub fn new() -> Self {
        Self {
            receipts: Vec::new(),
        }
    }

    /// Record a delivery receipt.
    pub fn record(&mut self, receipt: DeliveryReceipt) {
        self.receipts.push(receipt);
    }

    /// Returns the total number of receipts.
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// Returns true if the log is empty.
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }

    /// Returns all receipts for a given signal ID.
    pub fn receipts_for(&self, signal_id: &str) -> Vec<&DeliveryReceipt> {
        self.receipts
            .iter()
            .filter(|r| r.signal_id == signal_id)
            .collect()
    }

    /// Returns the count of successful deliveries.
    pub fn success_count(&self) -> usize {
        self.receipts.iter().filter(|r| r.is_success()).count()
    }

    /// Returns the count of failed deliveries.
    pub fn failure_count(&self) -> usize {
        self.receipts.iter().filter(|r| !r.is_success()).count()
    }

    /// Evict all receipts outside the hot retention window.
    pub fn evict_cold(&mut self) {
        self.receipts.retain(|r| r.is_hot());
    }
}

impl Default for DeliveryReceiptLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_success_detection() {
        let receipt = DeliveryReceipt::new("sig-1", DeliveryOutcome::Delivered, "fleet-handler");
        assert!(receipt.is_success());
        assert!(receipt.is_hot());
    }

    #[test]
    fn receipt_failure_detection() {
        let receipt = DeliveryReceipt::new(
            "sig-2",
            DeliveryOutcome::Failed {
                reason: "timeout".to_string(),
            },
            "fleet-handler",
        );
        assert!(!receipt.is_success());
    }

    #[test]
    fn receipt_log_accumulates() {
        let mut log = DeliveryReceiptLog::new();
        log.record(DeliveryReceipt::new(
            "sig-1",
            DeliveryOutcome::Delivered,
            "h1",
        ));
        log.record(DeliveryReceipt::new(
            "sig-2",
            DeliveryOutcome::Failed {
                reason: "err".to_string(),
            },
            "h2",
        ));
        assert_eq!(log.len(), 2);
        assert_eq!(log.success_count(), 1);
        assert_eq!(log.failure_count(), 1);
    }
}
