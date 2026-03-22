//! CostItem — one classified cost unit within a settlement record.
//! Every cost has a cause. Attribution reads this classification.

use serde::{Deserialize, Serialize};

/// What caused this cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CostClass {
    /// The core task execution (LLM calls, compute).
    DirectExecution,
    /// Cost incurred by approach rerouting (obstacles navigated).
    ReroutingOverhead { attempts: u32 },
    /// A sister system was called.
    SisterCall { sister_name: String },
    /// Knowledge gap was filled by hydra-omniscience.
    KnowledgeAcquisition { topic: String },
    /// Pre-execution red team analysis.
    RedTeamAnalysis,
    /// Wisdom synthesis (hydra-wisdom judgment).
    WisdomSynthesis,
    /// Scheduled background work.
    ScheduledWork { job_name: String },
    /// Skill action execution.
    SkillAction {
        skill_name: String,
        action_id: String,
    },
}

impl CostClass {
    pub fn label(&self) -> String {
        match self {
            Self::DirectExecution => "direct".into(),
            Self::ReroutingOverhead { .. } => "rerouting".into(),
            Self::SisterCall { sister_name } => format!("sister:{}", sister_name),
            Self::KnowledgeAcquisition { topic } => {
                format!("knowledge:{}", &topic[..topic.len().min(20)])
            }
            Self::RedTeamAnalysis => "redteam".into(),
            Self::WisdomSynthesis => "wisdom".into(),
            Self::ScheduledWork { job_name } => format!("scheduled:{}", job_name),
            Self::SkillAction { skill_name, .. } => format!("skill:{}", skill_name),
        }
    }

    pub fn is_overhead(&self) -> bool {
        matches!(self, Self::ReroutingOverhead { .. } | Self::RedTeamAnalysis)
    }
}

/// One classified cost item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostItem {
    pub id: String,
    pub class: CostClass,
    pub token_cost: f64,
    pub attention_cost: f64,
    pub time_cost: f64,
    pub total: f64,
}

impl CostItem {
    pub fn new(class: CostClass, tokens: u64, focus_units: f64, duration_ms: u64) -> Self {
        use crate::constants::*;
        let token_cost = (tokens as f64 / 1000.0) * COST_PER_1K_TOKENS;
        let attention_cost = focus_units * COST_PER_FOCUS_UNIT;
        let time_cost = (duration_ms as f64 / 1000.0) * COST_PER_SECOND;
        let total = token_cost + attention_cost + time_cost;

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            class,
            token_cost,
            attention_cost,
            time_cost,
            total,
        }
    }

    /// Rerouting adds an overhead multiplier.
    pub fn with_rerouting_overhead(mut self, attempts: u32) -> Self {
        if attempts > 0 {
            let overhead =
                self.total * crate::constants::REROUTING_OVERHEAD_MULTIPLIER * attempts as f64;
            self.total += overhead;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_computed_correctly() {
        let item = CostItem::new(
            CostClass::DirectExecution,
            2000, // 2K tokens
            10.0, // 10 focus units
            5000, // 5 seconds
        );
        // token: 2.0, attention: 1.0, time: 0.05
        assert!((item.token_cost - 2.0).abs() < 1e-10);
        assert!((item.attention_cost - 1.0).abs() < 1e-10);
        assert!((item.total - 3.05).abs() < 0.01);
    }

    #[test]
    fn rerouting_increases_total() {
        let base = CostItem::new(CostClass::DirectExecution, 1000, 5.0, 2000);
        let base_total = base.total;
        let with_rerout =
            CostItem::new(CostClass::DirectExecution, 1000, 5.0, 2000).with_rerouting_overhead(2);
        assert!(with_rerout.total > base_total);
    }

    #[test]
    fn overhead_classification() {
        let r = CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.0, 0);
        assert!(r.class.is_overhead());
        let d = CostItem::new(CostClass::DirectExecution, 0, 0.0, 0);
        assert!(!d.class.is_overhead());
    }
}
