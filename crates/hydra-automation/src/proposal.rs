//! CrystallizationProposal — pending user approval.
//! Hydra NEVER auto-crystallizes. Principal must approve.

use crate::pattern::BehaviorPattern;
use serde::{Deserialize, Serialize};

/// State of a crystallization proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalState {
    /// Awaiting principal decision.
    Pending,
    /// Principal approved — generating skill.
    Approved,
    /// Principal declined — pattern continues to be observed.
    Declined,
    /// Skill generated and hot-loaded.
    Crystallized { skill_name: String },
    /// Generation failed.
    Failed { reason: String },
}

impl ProposalState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::Crystallized { .. } => "crystallized",
            Self::Failed { .. } => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Declined | Self::Crystallized { .. } | Self::Failed { .. }
        )
    }
}

/// A crystallization proposal — what Hydra surfaces to the principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationProposal {
    pub id: String,
    pub pattern_id: String,
    pub action_id: String,
    pub domain: String,
    pub message: String,
    pub state: ProposalState,
    pub observation_count: usize,
    pub success_rate: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub decided_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CrystallizationProposal {
    pub fn from_pattern(pattern: &BehaviorPattern) -> Self {
        let message = format!(
            "I've seen '{}' {} times in the past {} days. \
             Success rate: {:.0}%. \
             Shall I crystallize this into a skill?",
            pattern.action_id,
            pattern.count,
            crate::constants::PATTERN_WINDOW_DAYS,
            pattern.success_rate * 100.0,
        );
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pattern_id: pattern.id.clone(),
            action_id: pattern.action_id.clone(),
            domain: pattern.domain.clone(),
            message,
            state: ProposalState::Pending,
            observation_count: pattern.count,
            success_rate: pattern.success_rate,
            created_at: chrono::Utc::now(),
            decided_at: None,
        }
    }

    pub fn approve(&mut self) {
        self.state = ProposalState::Approved;
        self.decided_at = Some(chrono::Utc::now());
    }

    pub fn decline(&mut self) {
        self.state = ProposalState::Declined;
        self.decided_at = Some(chrono::Utc::now());
    }

    pub fn mark_crystallized(&mut self, skill_name: impl Into<String>) {
        self.state = ProposalState::Crystallized {
            skill_name: skill_name.into(),
        };
    }

    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.state = ProposalState::Failed {
            reason: reason.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal() -> CrystallizationProposal {
        use crate::observation::ExecutionObservation;
        use std::collections::HashMap;

        let obs = ExecutionObservation::new(
            "deploy.staging",
            "deploy",
            HashMap::new(),
            "engineering",
            500,
            true,
        );
        let mut p = BehaviorPattern::new(&obs);
        for _ in 1..crate::constants::CRYSTALLIZATION_THRESHOLD {
            let o = ExecutionObservation::new(
                "deploy.staging",
                "deploy",
                HashMap::new(),
                "engineering",
                500,
                true,
            );
            p.add_observation(&o);
        }
        CrystallizationProposal::from_pattern(&p)
    }

    #[test]
    fn proposal_starts_pending() {
        let p = make_proposal();
        assert_eq!(p.state.label(), "pending");
        assert!(!p.state.is_terminal());
    }

    #[test]
    fn approve_changes_state() {
        let mut p = make_proposal();
        p.approve();
        assert_eq!(p.state.label(), "approved");
        assert!(p.decided_at.is_some());
    }

    #[test]
    fn decline_is_terminal() {
        let mut p = make_proposal();
        p.decline();
        assert!(p.state.is_terminal());
        assert_eq!(p.state.label(), "declined");
    }

    #[test]
    fn crystallized_is_terminal() {
        let mut p = make_proposal();
        p.approve();
        p.mark_crystallized("my-skill");
        assert!(p.state.is_terminal());
        assert_eq!(p.state.label(), "crystallized");
    }

    #[test]
    fn message_contains_count() {
        let p = make_proposal();
        assert!(p.message.contains("times"));
        assert!(p.message.contains("Shall I"));
    }
}
