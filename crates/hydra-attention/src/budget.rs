//! Attention budget computation and tracking.
//!
//! The budget determines how many cognitive units Hydra can spend
//! on the current input, based on intent and affect.

use crate::constants::{
    AFFECT_MULTIPLIER_CELEBRATORY, AFFECT_MULTIPLIER_CRISIS, AFFECT_MULTIPLIER_EXPLORATORY,
    AFFECT_MULTIPLIER_FRUSTRATED, AFFECT_MULTIPLIER_NEUTRAL, AFFECT_MULTIPLIER_UNDER_PRESSURE,
    BUDGET_ACTION_REQUEST, BUDGET_ANALYSIS_REQUEST, BUDGET_CONVERSATIONAL,
    BUDGET_GENERATIVE_REQUEST, BUDGET_INFORMATION_REQUEST, BUDGET_PLANNING_ASSIST,
    BUDGET_STATUS_QUERY, BUDGET_VERIFICATION_REQUEST, FULL_DEPTH_COST, SUMMARY_COST,
};
use crate::errors::AttentionError;
use hydra_language::{AffectSignal, IntentKind, InteractionRegister};
use serde::{Deserialize, Serialize};

/// The depth at which an item should be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingDepth {
    /// Full analysis — highest cognitive cost.
    Full,
    /// Summary-level processing — lower cost.
    Summary,
    /// Filtered out — zero cost, item is ignored.
    Filtered,
}

impl ProcessingDepth {
    /// Return the cognitive cost of this processing depth.
    pub fn cost(&self) -> u32 {
        match self {
            Self::Full => FULL_DEPTH_COST,
            Self::Summary => SUMMARY_COST,
            Self::Filtered => 0,
        }
    }
}

/// Tracks the attention budget for a single processing cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Total budget allocated for this cycle.
    total: u32,
    /// Budget consumed so far.
    consumed: u32,
}

impl AttentionBudget {
    /// Compute an attention budget from intent and affect.
    ///
    /// The base budget is determined by intent kind, then scaled
    /// by the affect multiplier. Crisis narrows the budget;
    /// exploratory widens it.
    pub fn compute(intent: &IntentKind, affect: &AffectSignal) -> Self {
        let base = base_budget(intent);
        let multiplier = affect_multiplier(&affect.register);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total = (base as f64 * multiplier).round() as u32;
        Self { total, consumed: 0 }
    }

    /// Consume budget for a given processing depth.
    ///
    /// Returns an error if the budget would be exceeded.
    pub fn consume(&mut self, depth: ProcessingDepth) -> Result<(), AttentionError> {
        let cost = depth.cost();
        if self.consumed + cost > self.total {
            return Err(AttentionError::BudgetExhausted(format!(
                "need {} but only {} remaining",
                cost,
                self.remaining()
            )));
        }
        self.consumed += cost;
        Ok(())
    }

    /// Return the remaining budget.
    pub fn remaining(&self) -> u32 {
        self.total.saturating_sub(self.consumed)
    }

    /// Check whether the budget can afford a given processing depth.
    pub fn can_afford(&self, depth: ProcessingDepth) -> bool {
        self.remaining() >= depth.cost()
    }

    /// Return the utilization ratio (consumed / total).
    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.consumed as f64 / self.total as f64
    }

    /// Return the total budget.
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Return the consumed budget.
    pub fn consumed(&self) -> u32 {
        self.consumed
    }
}

/// Look up the base budget for a given intent kind.
fn base_budget(intent: &IntentKind) -> u32 {
    match intent {
        IntentKind::ActionRequest => BUDGET_ACTION_REQUEST,
        IntentKind::AnalysisRequest => BUDGET_ANALYSIS_REQUEST,
        IntentKind::VerificationRequest => BUDGET_VERIFICATION_REQUEST,
        IntentKind::PlanningAssist => BUDGET_PLANNING_ASSIST,
        IntentKind::GenerativeRequest => BUDGET_GENERATIVE_REQUEST,
        IntentKind::Conversational => BUDGET_CONVERSATIONAL,
        IntentKind::StatusQuery => BUDGET_STATUS_QUERY,
        IntentKind::InformationRequest => BUDGET_INFORMATION_REQUEST,
    }
}

/// Look up the affect multiplier for an interaction register.
fn affect_multiplier(register: &InteractionRegister) -> f64 {
    match register {
        InteractionRegister::Neutral => AFFECT_MULTIPLIER_NEUTRAL,
        InteractionRegister::UnderPressure => AFFECT_MULTIPLIER_UNDER_PRESSURE,
        InteractionRegister::Frustrated => AFFECT_MULTIPLIER_FRUSTRATED,
        InteractionRegister::Crisis => AFFECT_MULTIPLIER_CRISIS,
        InteractionRegister::Celebratory => AFFECT_MULTIPLIER_CELEBRATORY,
        InteractionRegister::Exploratory => AFFECT_MULTIPLIER_EXPLORATORY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neutral_affect() -> AffectSignal {
        AffectSignal {
            register: InteractionRegister::Neutral,
            confidence: 0.7,
            keywords_detected: vec![],
        }
    }

    #[test]
    fn analysis_budget_larger_than_status() {
        let analysis = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral_affect());
        let status = AttentionBudget::compute(&IntentKind::StatusQuery, &neutral_affect());
        assert!(analysis.total() > status.total());
    }

    #[test]
    fn crisis_reduces_budget() {
        let crisis_affect = AffectSignal {
            register: InteractionRegister::Crisis,
            confidence: 0.9,
            keywords_detected: vec!["broken".into()],
        };
        let neutral = AttentionBudget::compute(&IntentKind::ActionRequest, &neutral_affect());
        let crisis = AttentionBudget::compute(&IntentKind::ActionRequest, &crisis_affect);
        assert!(crisis.total() < neutral.total());
    }

    #[test]
    fn consumption_tracking() {
        let mut budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral_affect());
        let initial = budget.remaining();
        assert!(budget.consume(ProcessingDepth::Full).is_ok());
        assert_eq!(budget.remaining(), initial - FULL_DEPTH_COST);
    }

    #[test]
    fn cannot_exceed_budget() {
        let mut budget = AttentionBudget {
            total: 5,
            consumed: 0,
        };
        assert!(budget.consume(ProcessingDepth::Full).is_err());
    }

    #[test]
    fn filtered_costs_nothing() {
        assert_eq!(ProcessingDepth::Filtered.cost(), 0);
    }
}
