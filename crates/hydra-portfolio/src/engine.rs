//! PortfolioEngine — resource allocation coordinator.

use crate::{
    allocation::ResourceAllocation,
    constants::MAX_PORTFOLIO_OBJECTIVES,
    errors::PortfolioError,
    objective::{ObjectiveCategory, PortfolioObjective},
    ranker::rank_objectives,
};
use hydra_attribution::AvoidabilityReport;

/// The portfolio engine.
pub struct PortfolioEngine {
    objectives: Vec<PortfolioObjective>,
}

impl PortfolioEngine {
    pub fn new() -> Self {
        Self {
            objectives: Vec::new(),
        }
    }

    /// Add an objective to the portfolio.
    pub fn add_objective(&mut self, obj: PortfolioObjective) -> Result<(), PortfolioError> {
        if self.objectives.len() >= MAX_PORTFOLIO_OBJECTIVES {
            return Err(PortfolioError::PortfolioFull {
                max: MAX_PORTFOLIO_OBJECTIVES,
            });
        }
        self.objectives.push(obj);
        Ok(())
    }

    /// Inject objectives from avoidability report
    /// (converts avoidable costs into cost-reduction objectives).
    pub fn ingest_avoidability(&mut self, report: &AvoidabilityReport) {
        if report.avoidable_fraction < 0.10 {
            return;
        }

        for (cause, cost) in &report.top_avoidable {
            let urgency = if report.alert { 0.85 } else { 0.50 };
            let obj = PortfolioObjective::new(
                format!("Reduce avoidable cost: {}", cause),
                format!(
                    "Attribution identified {:.1} in avoidable cost from {}. \
                     Recommendation: {}",
                    cost,
                    cause,
                    report
                        .recommendations
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "review operational patterns".into()),
                ),
                ObjectiveCategory::CostReduction,
                0.0,                             // no risk if ignored — just waste
                report.avoidable_fraction * 0.8, // ROI from reducing it
                0.5,                             // neutral orientation alignment
                urgency,
                10.0,
            );
            if self.objectives.len() < MAX_PORTFOLIO_OBJECTIVES {
                self.objectives.push(obj);
            }
        }
    }

    /// Generate allocation for the current period.
    pub fn allocate(
        &self,
        budget: f64,
        period_label: impl Into<String>,
    ) -> Result<ResourceAllocation, PortfolioError> {
        if self.objectives.is_empty() {
            return Err(PortfolioError::NoObjectives);
        }
        let ranked = rank_objectives(&self.objectives);
        Ok(ResourceAllocation::from_ranked(
            &ranked,
            budget,
            period_label,
        ))
    }

    pub fn objective_count(&self) -> usize {
        self.objectives.len()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!("portfolio: objectives={}", self.objective_count())
    }
}

impl Default for PortfolioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_ATTENTION_BUDGET;

    fn security_obj(urgency: f64) -> PortfolioObjective {
        PortfolioObjective::new(
            "Fix auth surface",
            "Harden auth",
            ObjectiveCategory::SecurityHardening,
            0.85,
            0.75,
            0.80,
            urgency,
            20.0,
        )
    }

    #[test]
    fn allocate_produces_recommendation() {
        let mut engine = PortfolioEngine::new();
        engine
            .add_objective(security_obj(0.90))
            .expect("add security");
        engine
            .add_objective(PortfolioObjective::new(
                "Optimize query",
                "Improve DB perf",
                ObjectiveCategory::PerformanceOptimization,
                0.20,
                0.60,
                0.50,
                0.40,
                10.0,
            ))
            .expect("add perf");
        let alloc = engine
            .allocate(DEFAULT_ATTENTION_BUDGET, "Q2-2026")
            .expect("allocate");
        assert!(!alloc.allocations.is_empty());
        assert!(alloc.top_recommendation().is_some());
    }

    #[test]
    fn high_risk_high_urgency_ranks_first() {
        let mut engine = PortfolioEngine::new();
        engine
            .add_objective(security_obj(0.95))
            .expect("add security");
        engine
            .add_objective(PortfolioObjective::new(
                "Nice to have",
                "Low priority work",
                ObjectiveCategory::MaintenanceAndDebt,
                0.10,
                0.20,
                0.30,
                0.20,
                5.0,
            ))
            .expect("add maintenance");
        let alloc = engine
            .allocate(DEFAULT_ATTENTION_BUDGET, "Q2")
            .expect("allocate");
        let top = alloc.top_recommendation().expect("top");
        assert_eq!(top.name, "Fix auth surface");
    }

    #[test]
    fn empty_portfolio_error() {
        let engine = PortfolioEngine::new();
        let result = engine.allocate(100.0, "Q1");
        assert!(matches!(result, Err(PortfolioError::NoObjectives)));
    }
}
