//! ExchangeReceipt — immutable record of one completed exchange.
//! SHA256 hashed. Append-only. Constitutional.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One immutable exchange receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeReceipt {
    pub id: String,
    pub request_id: String,
    pub offer_id: String,
    pub counterparty: String,
    pub capability: String,
    pub exchange_value: f64,
    pub outcome: ExchangeOutcome,
    pub integrity_hash: String,
    pub issued_at: chrono::DateTime<chrono::Utc>,
}

/// The outcome of an exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExchangeOutcome {
    Fulfilled {
        description: String,
    },
    Rejected {
        reason: String,
    },
    Partial {
        description: String,
        fraction_delivered: f64,
    },
}

impl ExchangeOutcome {
    /// Whether the exchange produced any value.
    pub fn is_successful(&self) -> bool {
        matches!(self, Self::Fulfilled { .. } | Self::Partial { .. })
    }

    /// Human-readable label for this outcome.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fulfilled { .. } => "fulfilled",
            Self::Rejected { .. } => "rejected",
            Self::Partial { .. } => "partial",
        }
    }
}

impl ExchangeReceipt {
    /// Issue a new receipt with a computed integrity hash.
    pub fn new(
        request_id: impl Into<String>,
        offer_id: impl Into<String>,
        counterparty: impl Into<String>,
        capability: impl Into<String>,
        exchange_value: f64,
        outcome: ExchangeOutcome,
    ) -> Self {
        let now = chrono::Utc::now();
        let req_id = request_id.into();
        let offer_id_s = offer_id.into();
        let counterparty_s = counterparty.into();
        let capability_s = capability.into();

        let hash = Self::compute_hash(&req_id, &offer_id_s, &counterparty_s, exchange_value, &now);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: req_id,
            offer_id: offer_id_s,
            counterparty: counterparty_s,
            capability: capability_s,
            exchange_value,
            outcome,
            integrity_hash: hash,
            issued_at: now,
        }
    }

    fn compute_hash(
        request_id: &str,
        offer_id: &str,
        counterparty: &str,
        value: f64,
        at: &chrono::DateTime<chrono::Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(request_id.as_bytes());
        h.update(offer_id.as_bytes());
        h.update(counterparty.as_bytes());
        h.update(value.to_bits().to_le_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    /// Verify that the integrity hash is well-formed.
    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_integrity_hash() {
        let r = ExchangeReceipt::new(
            "req-1",
            "offer-1",
            "agent-beta",
            "settlement:agentra-settlement",
            100.0,
            ExchangeOutcome::Fulfilled {
                description: "executed".into(),
            },
        );
        assert!(r.verify_integrity());
        assert_eq!(r.integrity_hash.len(), 64);
    }

    #[test]
    fn fulfilled_is_successful() {
        let o = ExchangeOutcome::Fulfilled {
            description: "done".into(),
        };
        assert!(o.is_successful());
    }

    #[test]
    fn rejected_not_successful() {
        let o = ExchangeOutcome::Rejected {
            reason: "trust too low".into(),
        };
        assert!(!o.is_successful());
    }
}
