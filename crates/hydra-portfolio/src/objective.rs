//! PortfolioObjective — one competing goal for Hydra's attention.

use serde::{Deserialize, Serialize};

/// What category of work this objective represents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveCategory {
    SecurityHardening,
    PerformanceOptimization,
    CostReduction,
    CapabilityExpansion,
    KnowledgeAcquisition,
    MaintenanceAndDebt,
    BusinessExecution,   // e.g. running the settlement system
    OrientationCritical, // soul-flagged as foundational
}

impl ObjectiveCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SecurityHardening => "security",
            Self::PerformanceOptimization => "performance",
            Self::CostReduction => "cost-reduction",
            Self::CapabilityExpansion => "capability",
            Self::KnowledgeAcquisition => "knowledge",
            Self::MaintenanceAndDebt => "maintenance",
            Self::BusinessExecution => "business",
            Self::OrientationCritical => "orientation-critical",
        }
    }
}

/// One objective in the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioObjective {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ObjectiveCategory,
    /// Risk score if this is NOT addressed (0.0–1.0).
    pub risk_if_ignored: f64,
    /// Estimated ROI from addressing this (0.0–1.0, relative).
    pub estimated_roi: f64,
    /// How aligned is this with current orientation? (0.0–1.0).
    pub orientation_score: f64,
    /// How urgent is this? (0.0–1.0, 1.0 = must do now).
    pub urgency: f64,
    /// Estimated attention budget required.
    pub attention_required: f64,
    pub added_at: chrono::DateTime<chrono::Utc>,
}

impl PortfolioObjective {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: ObjectiveCategory,
        risk_if_ignored: f64,
        estimated_roi: f64,
        orientation_score: f64,
        urgency: f64,
        attention_required: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            category,
            risk_if_ignored: risk_if_ignored.clamp(0.0, 1.0),
            estimated_roi: estimated_roi.clamp(0.0, 1.0),
            orientation_score: orientation_score.clamp(0.0, 1.0),
            urgency: urgency.clamp(0.0, 1.0),
            attention_required: attention_required.max(0.0),
            added_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_scores_clamped() {
        let o = PortfolioObjective::new(
            "test",
            "desc",
            ObjectiveCategory::SecurityHardening,
            1.5,
            -0.1,
            0.8,
            0.7,
            10.0,
        );
        assert_eq!(o.risk_if_ignored, 1.0);
        assert_eq!(o.estimated_roi, 0.0);
    }
}
