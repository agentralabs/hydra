//! ExchangeRequest — what Hydra or an external system needs.

use crate::offer::OfferKind;
use serde::{Deserialize, Serialize};

/// The state of an exchange request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestState {
    Pending,
    Approved,
    Executing,
    Fulfilled { receipt_id: String },
    Rejected { reason: String },
    Escalated { reason: String },
}

impl RequestState {
    /// Whether the request has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Fulfilled { .. } | Self::Rejected { .. })
    }

    /// Human-readable label for this state.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Executing => "executing",
            Self::Fulfilled { .. } => "fulfilled",
            Self::Rejected { .. } => "rejected",
            Self::Escalated { .. } => "escalated",
        }
    }
}

/// One exchange request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRequest {
    pub id: String,
    pub counterparty: String,
    pub capability: OfferKind,
    pub context: String,
    pub state: RequestState,
    pub trust_score: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ExchangeRequest {
    /// Create a new pending exchange request.
    pub fn new(
        counterparty: impl Into<String>,
        capability: OfferKind,
        context: impl Into<String>,
        trust_score: f64,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            counterparty: counterparty.into(),
            capability,
            context: context.into(),
            state: RequestState::Pending,
            trust_score: trust_score.clamp(0.0, 1.0),
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to Approved.
    pub fn approve(&mut self) {
        self.state = RequestState::Approved;
        self.updated_at = chrono::Utc::now();
    }

    /// Transition to Fulfilled with a receipt id.
    pub fn fulfill(&mut self, receipt_id: impl Into<String>) {
        self.state = RequestState::Fulfilled {
            receipt_id: receipt_id.into(),
        };
        self.updated_at = chrono::Utc::now();
    }

    /// Transition to Rejected with a reason.
    pub fn reject(&mut self, reason: impl Into<String>) {
        self.state = RequestState::Rejected {
            reason: reason.into(),
        };
        self.updated_at = chrono::Utc::now();
    }

    /// Transition to Escalated with a reason.
    pub fn escalate(&mut self, reason: impl Into<String>) {
        self.state = RequestState::Escalated {
            reason: reason.into(),
        };
        self.updated_at = chrono::Utc::now();
    }

    /// Whether the counterparty's trust meets the given threshold.
    pub fn meets_trust_threshold(&self, min_trust: f64) -> bool {
        self.trust_score >= min_trust
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_state_lifecycle() {
        let mut r = ExchangeRequest::new(
            "agent-beta",
            OfferKind::RedTeamAnalysis,
            "analyze deployment plan",
            0.75,
        );
        assert_eq!(r.state.label(), "pending");
        r.approve();
        assert_eq!(r.state.label(), "approved");
        r.fulfill("receipt-123");
        assert!(r.state.is_terminal());
        assert_eq!(r.state.label(), "fulfilled");
    }

    #[test]
    fn trust_threshold_check() {
        let r = ExchangeRequest::new("agent", OfferKind::RedTeamAnalysis, "context", 0.55);
        assert!(!r.meets_trust_threshold(0.60));
        assert!(r.meets_trust_threshold(0.50));
    }
}
