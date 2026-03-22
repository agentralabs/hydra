//! ResourceAllocation — the portfolio recommendation output.

use crate::ranker::ScoredObjective;
use serde::{Deserialize, Serialize};

/// How attention budget is allocated across objectives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationEntry {
    pub objective_id: String,
    pub name: String,
    pub score: f64,
    pub allocated_pct: f64,   // percentage of total attention budget
    pub allocated_units: f64, // absolute attention units
    pub rationale: String,
}

/// The full portfolio allocation recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub total_budget: f64,
    pub allocations: Vec<AllocationEntry>,
    pub unallocated_pct: f64,
    pub period_label: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl ResourceAllocation {
    /// Build allocation from ranked objectives and total budget.
    pub fn from_ranked(
        ranked: &[ScoredObjective],
        total_budget: f64,
        period_label: impl Into<String>,
    ) -> Self {
        if ranked.is_empty() {
            return Self::empty(total_budget, period_label);
        }

        // Score-proportional allocation
        let total_score: f64 = ranked.iter().map(|o| o.score).sum();
        let mut allocations = Vec::new();
        let mut allocated = 0.0_f64;

        for obj in ranked {
            let pct = (obj.score / total_score) * 100.0;
            let units = (pct / 100.0) * total_budget;
            allocated += pct;
            allocations.push(AllocationEntry {
                objective_id: obj.objective_id.clone(),
                name: obj.name.clone(),
                score: obj.score,
                allocated_pct: pct,
                allocated_units: units,
                rationale: obj.rationale.clone(),
            });
        }

        Self {
            total_budget,
            allocations,
            unallocated_pct: (100.0 - allocated).max(0.0),
            period_label: period_label.into(),
            generated_at: chrono::Utc::now(),
        }
    }

    fn empty(budget: f64, label: impl Into<String>) -> Self {
        Self {
            total_budget: budget,
            allocations: vec![],
            unallocated_pct: 100.0,
            period_label: label.into(),
            generated_at: chrono::Utc::now(),
        }
    }

    /// Top recommended objective.
    pub fn top_recommendation(&self) -> Option<&AllocationEntry> {
        self.allocations.first()
    }

    /// Human-readable brief.
    pub fn brief(&self) -> String {
        let top = self
            .top_recommendation()
            .map(|a| format!("{} ({:.0}%)", a.name, a.allocated_pct))
            .unwrap_or_else(|| "none".into());
        format!(
            "Portfolio ({}): {} objectives | top: {} | unallocated: {:.0}%",
            self.period_label,
            self.allocations.len(),
            top,
            self.unallocated_pct,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scored(score: f64, name: &str) -> ScoredObjective {
        ScoredObjective {
            objective_id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            category: "security".to_string(),
            score,
            rationale: "test".to_string(),
        }
    }

    #[test]
    fn allocation_sums_to_100() {
        let ranked = vec![scored(0.8, "A"), scored(0.6, "B"), scored(0.4, "C")];
        let alloc = ResourceAllocation::from_ranked(&ranked, 100.0, "Q1");
        let total: f64 = alloc.allocations.iter().map(|a| a.allocated_pct).sum();
        assert!((total - 100.0).abs() < 0.01);
    }

    #[test]
    fn highest_score_gets_most_allocation() {
        let ranked = vec![scored(0.9, "high"), scored(0.3, "low")];
        let alloc = ResourceAllocation::from_ranked(&ranked, 100.0, "Q1");
        assert!(alloc.allocations[0].allocated_pct > alloc.allocations[1].allocated_pct);
    }
}
