//! TrustScope — what two peers agree to share.
//! Negotiated before any sharing begins.
//! Neither party can unilaterally expand scope.
//! Scope violations are hard stops.

use serde::{Deserialize, Serialize};

/// One item in a trust scope — what can be shared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScopeItem {
    /// Share genome entries for a specific domain.
    GenomeEntries { domain: String, max_count: usize },
    /// Share wisdom judgments for a domain.
    WisdomJudgments { domain: String },
    /// Share a specific artifact kind.
    Artifacts { kind: String },
    /// Participate in collective pattern detection.
    PatternDetection,
    /// Share calibration data (domain bias profiles).
    CalibrationData { domain: String },
}

impl ScopeItem {
    pub fn label(&self) -> String {
        match self {
            Self::GenomeEntries { domain, .. } => format!("genome:{}", domain),
            Self::WisdomJudgments { domain } => format!("wisdom:{}", domain),
            Self::Artifacts { kind } => format!("artifact:{}", kind),
            Self::PatternDetection => "pattern".into(),
            Self::CalibrationData { domain } => format!("calibration:{}", domain),
        }
    }
}

/// The state of a trust negotiation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NegotiationState {
    /// We have proposed a scope, awaiting their agreement.
    ProposedByUs {
        proposed_at: chrono::DateTime<chrono::Utc>,
    },
    /// They have proposed, we are reviewing.
    ProposedByThem {
        proposed_at: chrono::DateTime<chrono::Utc>,
    },
    /// Both sides agreed — scope is active.
    Agreed {
        agreed_at: chrono::DateTime<chrono::Utc>,
    },
    /// Negotiation failed — no agreement.
    Failed { reason: String },
    /// Scope was revoked by one party.
    Revoked {
        by: String,
        at: chrono::DateTime<chrono::Utc>,
    },
}

impl NegotiationState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Agreed { .. })
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::ProposedByUs { .. } => "proposed-us",
            Self::ProposedByThem { .. } => "proposed-them",
            Self::Agreed { .. } => "agreed",
            Self::Failed { .. } => "failed",
            Self::Revoked { .. } => "revoked",
        }
    }
}

/// An agreed trust scope between two peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScope {
    pub id: String,
    pub local_peer_id: String,
    pub remote_peer_id: String,
    /// What WE offer to share.
    pub our_offers: Vec<ScopeItem>,
    /// What THEY offer to share.
    pub their_offers: Vec<ScopeItem>,
    pub state: NegotiationState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TrustScope {
    pub fn new(
        local_peer_id: impl Into<String>,
        remote_peer_id: impl Into<String>,
        our_offers: Vec<ScopeItem>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            local_peer_id: local_peer_id.into(),
            remote_peer_id: remote_peer_id.into(),
            our_offers,
            their_offers: Vec::new(),
            state: NegotiationState::ProposedByUs {
                proposed_at: chrono::Utc::now(),
            },
            created_at: chrono::Utc::now(),
        }
    }

    /// Peer responds with their offer — negotiate toward agreement.
    pub fn counter_offer(&mut self, their_offers: Vec<ScopeItem>) {
        self.their_offers = their_offers;
        // Simple agreement: if both sides offer at least one item -> agreed
        if !self.our_offers.is_empty() && !self.their_offers.is_empty() {
            self.state = NegotiationState::Agreed {
                agreed_at: chrono::Utc::now(),
            };
        } else {
            self.state = NegotiationState::Failed {
                reason: "One or both sides offered nothing".into(),
            };
        }
    }

    /// Check if a specific sharing action is within scope.
    pub fn permits(&self, action: &str) -> bool {
        if !self.state.is_active() {
            return false;
        }
        self.our_offers
            .iter()
            .any(|item| action.contains(&item.label()))
            || self
                .their_offers
                .iter()
                .any(|item| action.contains(&item.label()))
    }

    pub fn revoke(&mut self, by: &str) {
        self.state = NegotiationState::Revoked {
            by: by.to_string(),
            at: chrono::Utc::now(),
        };
    }

    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreement_reached_when_both_offer() {
        let mut scope = TrustScope::new(
            "hydra-a",
            "hydra-b",
            vec![ScopeItem::GenomeEntries {
                domain: "engineering".into(),
                max_count: 50,
            }],
        );
        scope.counter_offer(vec![ScopeItem::GenomeEntries {
            domain: "fintech".into(),
            max_count: 30,
        }]);
        assert!(scope.is_active());
        assert_eq!(scope.state.label(), "agreed");
    }

    #[test]
    fn no_agreement_when_empty_counter() {
        let mut scope = TrustScope::new("a", "b", vec![ScopeItem::PatternDetection]);
        scope.counter_offer(vec![]); // empty counter
        assert!(!scope.is_active());
        assert_eq!(scope.state.label(), "failed");
    }

    #[test]
    fn permits_within_scope() {
        let mut scope = TrustScope::new(
            "a",
            "b",
            vec![ScopeItem::GenomeEntries {
                domain: "engineering".into(),
                max_count: 10,
            }],
        );
        scope.counter_offer(vec![ScopeItem::PatternDetection]);
        assert!(scope.permits("genome:engineering"));
        assert!(!scope.permits("genome:fintech")); // not in scope
    }

    #[test]
    fn revoked_scope_not_active() {
        let mut scope = TrustScope::new("a", "b", vec![ScopeItem::PatternDetection]);
        scope.counter_offer(vec![ScopeItem::PatternDetection]);
        assert!(scope.is_active());
        scope.revoke("hydra-a");
        assert!(!scope.is_active());
    }
}
