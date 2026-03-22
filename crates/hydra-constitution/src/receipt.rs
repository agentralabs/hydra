//! Receipt and ReceiptChain types.
//! Receipts are the immutable audit trail of everything Hydra does.

use crate::constants::{CONSTITUTIONAL_IDENTITY_ID, RECEIPT_CHAIN_MAX_DEPTH};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique, immutable identifier for a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReceiptId(String);

impl ReceiptId {
    /// Generate a new unique receipt ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// The constitutional root receipt ID.
    pub fn constitutional_identity() -> Self {
        Self(CONSTITUTIONAL_IDENTITY_ID.to_string())
    }

    /// Returns the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if this is the constitutional identity receipt.
    pub fn is_constitutional_identity(&self) -> bool {
        self.0 == CONSTITUTIONAL_IDENTITY_ID
    }
}

impl Default for ReceiptId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An immutable record of one action Hydra took.
/// Once created, a receipt cannot be modified. Ever. (Law 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Unique ID for this receipt.
    pub id: ReceiptId,

    /// When this action occurred.
    pub timestamp: DateTime<Utc>,

    /// The action that was taken.
    pub action_type: String,

    /// The target of the action.
    pub target: String,

    /// The receipt ID of the action that caused this one.
    pub caused_by: ReceiptId,

    /// Which trust tier the acting entity was at.
    pub source_tier: u8,

    /// Short description of the outcome.
    pub outcome: String,

    /// Whether the action succeeded.
    pub succeeded: bool,
}

impl Receipt {
    /// Create a new receipt for an action.
    pub fn new(
        action_type: impl Into<String>,
        target: impl Into<String>,
        caused_by: ReceiptId,
        source_tier: u8,
        outcome: impl Into<String>,
        succeeded: bool,
    ) -> Self {
        Self {
            id: ReceiptId::new(),
            timestamp: Utc::now(),
            action_type: action_type.into(),
            target: target.into(),
            caused_by,
            source_tier,
            outcome: outcome.into(),
            succeeded,
        }
    }
}

/// A chain of receipts forming an immutable causal history.
/// Monotonically increasing — receipts are only ever added. (Law 1)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReceiptChain {
    receipts: Vec<Receipt>,
}

impl ReceiptChain {
    /// Create a new empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a receipt. Receipts are always appended, never inserted or modified.
    pub fn append(&mut self, receipt: Receipt) -> &ReceiptId {
        self.receipts.push(receipt);
        &self.receipts.last().unwrap().id
    }

    /// Total number of receipts in the chain.
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// Returns true if the chain has no receipts.
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }

    /// Find a receipt by its ID.
    pub fn find(&self, id: &ReceiptId) -> Option<&Receipt> {
        self.receipts.iter().find(|r| &r.id == id)
    }

    /// The most recent receipt, if any.
    pub fn latest(&self) -> Option<&Receipt> {
        self.receipts.last()
    }

    /// Returns true if adding another receipt would exceed the max depth.
    pub fn is_at_max_depth(&self) -> bool {
        self.receipts.len() >= RECEIPT_CHAIN_MAX_DEPTH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_id_is_unique() {
        let a = ReceiptId::new();
        let b = ReceiptId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn constitutional_identity_is_stable() {
        let a = ReceiptId::constitutional_identity();
        let b = ReceiptId::constitutional_identity();
        assert_eq!(a, b);
        assert!(a.is_constitutional_identity());
    }

    #[test]
    fn chain_grows_monotonically() {
        let mut chain = ReceiptChain::new();
        assert_eq!(chain.len(), 0);

        let r = Receipt::new(
            "test.action",
            "target-a",
            ReceiptId::constitutional_identity(),
            3,
            "success",
            true,
        );
        chain.append(r);
        assert_eq!(chain.len(), 1);

        let r2 = Receipt::new(
            "test.action2",
            "target-b",
            ReceiptId::constitutional_identity(),
            3,
            "success",
            true,
        );
        chain.append(r2);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn chain_find_works() {
        let mut chain = ReceiptChain::new();
        let r = Receipt::new(
            "test.action",
            "target",
            ReceiptId::constitutional_identity(),
            3,
            "ok",
            true,
        );
        let id = r.id.clone();
        chain.append(r);
        assert!(chain.find(&id).is_some());
    }
}
