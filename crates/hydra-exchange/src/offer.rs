//! ExchangeOffer — what Hydra is willing to provide to others.
//! Trust-gated. Cost-declared. Condition-specified.

use serde::{Deserialize, Serialize};

/// What type of capability is being offered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OfferKind {
    /// Execute a skill action on behalf of the requester.
    SkillExecution {
        skill_name: String,
        action_id: String,
    },
    /// Share genome entries for a domain.
    GenomeSharing { domain: String, max_entries: usize },
    /// Share a crystallized artifact.
    ArtifactSharing { artifact_kind: String },
    /// Provide a wisdom judgment for a context.
    WisdomJudgment { domain: String },
    /// Provide red team analysis.
    RedTeamAnalysis,
    /// Provide settlement execution capability.
    SettlementExecution { skill_name: String },
}

impl OfferKind {
    /// Return a colon-separated label for this kind.
    pub fn label(&self) -> String {
        match self {
            Self::SkillExecution {
                skill_name,
                action_id,
            } => format!("skill:{}:{}", skill_name, action_id),
            Self::GenomeSharing { domain, .. } => format!("genome:{}", domain),
            Self::ArtifactSharing { artifact_kind } => format!("artifact:{}", artifact_kind),
            Self::WisdomJudgment { domain } => format!("wisdom:{}", domain),
            Self::RedTeamAnalysis => "redteam".into(),
            Self::SettlementExecution { skill_name } => format!("settlement:{}", skill_name),
        }
    }
}

/// The state of an offer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OfferState {
    Active,
    Suspended { reason: String },
    Withdrawn,
    Exhausted,
}

impl OfferState {
    /// Whether this state allows fulfillment.
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// One exchange offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeOffer {
    pub id: String,
    pub kind: OfferKind,
    pub description: String,
    /// Minimum trust score required from the requester.
    pub min_trust_required: f64,
    /// Cost in settlement units per exchange.
    pub cost_per_exchange: f64,
    /// Maximum number of times this can be fulfilled.
    pub max_fulfillments: Option<usize>,
    pub fulfillment_count: usize,
    pub state: OfferState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ExchangeOffer {
    /// Create a new exchange offer.
    pub fn new(
        kind: OfferKind,
        description: impl Into<String>,
        min_trust_required: f64,
        cost_per_exchange: f64,
        max_fulfillments: Option<usize>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            description: description.into(),
            min_trust_required: min_trust_required.clamp(0.0, 1.0),
            cost_per_exchange: cost_per_exchange.max(0.0),
            max_fulfillments,
            fulfillment_count: 0,
            state: OfferState::Active,
            created_at: chrono::Utc::now(),
        }
    }

    /// Whether this offer can still be fulfilled.
    pub fn is_available(&self) -> bool {
        self.state.is_available()
            && self
                .max_fulfillments
                .map(|max| self.fulfillment_count < max)
                .unwrap_or(true)
    }

    /// Record one fulfillment; exhaust if at capacity.
    pub fn fulfill(&mut self) {
        self.fulfillment_count += 1;
        if let Some(max) = self.max_fulfillments {
            if self.fulfillment_count >= max {
                self.state = OfferState::Exhausted;
            }
        }
    }

    /// Mark the offer as withdrawn.
    pub fn withdraw(&mut self) {
        self.state = OfferState::Withdrawn;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_available_until_exhausted() {
        let mut offer = ExchangeOffer::new(
            OfferKind::RedTeamAnalysis,
            "Red team analysis",
            0.70,
            10.0,
            Some(3),
        );
        assert!(offer.is_available());
        offer.fulfill();
        offer.fulfill();
        offer.fulfill();
        assert!(!offer.is_available());
        assert_eq!(offer.state, OfferState::Exhausted);
    }

    #[test]
    fn unlimited_offer_never_exhausted() {
        let mut offer = ExchangeOffer::new(
            OfferKind::WisdomJudgment {
                domain: "fintech".into(),
            },
            "Wisdom",
            0.60,
            5.0,
            None,
        );
        for _ in 0..100 {
            offer.fulfill();
        }
        assert!(offer.is_available());
    }

    #[test]
    fn withdrawn_not_available() {
        let mut offer = ExchangeOffer::new(
            OfferKind::GenomeSharing {
                domain: "test".into(),
                max_entries: 10,
            },
            "Genome",
            0.65,
            2.0,
            None,
        );
        offer.withdraw();
        assert!(!offer.is_available());
    }
}
