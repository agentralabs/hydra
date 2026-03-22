//! ObjectiveRanker — score and order portfolio objectives.

use crate::{constants::*, objective::PortfolioObjective};
use serde::{Deserialize, Serialize};

/// A scored objective ready for allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredObjective {
    pub objective_id: String,
    pub name: String,
    pub category: String,
    pub score: f64,
    pub rationale: String,
}

impl ScoredObjective {
    pub fn from_objective(obj: &PortfolioObjective) -> Self {
        let score = compute_score(obj);
        let rationale = build_rationale(obj, score);
        Self {
            objective_id: obj.id.clone(),
            name: obj.name.clone(),
            category: obj.category.label().to_string(),
            score,
            rationale,
        }
    }
}

fn compute_score(obj: &PortfolioObjective) -> f64 {
    let risk_component = obj.risk_if_ignored * WEIGHT_RISK_REDUCTION;
    let orientation_component = obj.orientation_score * WEIGHT_ORIENTATION;
    let roi_component = obj.estimated_roi * WEIGHT_ROI;
    let urgency_component = obj.urgency * WEIGHT_URGENCY;
    (risk_component + orientation_component + roi_component + urgency_component).clamp(0.0, 1.0)
}

fn build_rationale(obj: &PortfolioObjective, score: f64) -> String {
    let mut parts = Vec::new();
    if obj.risk_if_ignored >= 0.70 {
        parts.push(format!(
            "HIGH risk if ignored ({:.0}%)",
            obj.risk_if_ignored * 100.0
        ));
    }
    if obj.orientation_score >= 0.75 {
        parts.push("strongly aligned with orientation".into());
    }
    if obj.urgency >= 0.80 {
        parts.push("urgent".into());
    }
    if obj.estimated_roi >= 0.70 {
        parts.push(format!("strong ROI ({:.0}%)", obj.estimated_roi * 100.0));
    }
    if parts.is_empty() {
        parts.push("standard priority".into());
    }
    format!("Score {:.2}: {}", score, parts.join(", "))
}

/// Rank a list of objectives by score.
pub fn rank_objectives(objectives: &[PortfolioObjective]) -> Vec<ScoredObjective> {
    let mut scored: Vec<ScoredObjective> = objectives
        .iter()
        .filter(|o| compute_score(o) >= MIN_RECOMMENDATION_SCORE)
        .map(ScoredObjective::from_objective)
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objective::ObjectiveCategory;

    fn obj(risk: f64, orientation: f64, roi: f64, urgency: f64) -> PortfolioObjective {
        PortfolioObjective::new(
            "test",
            "desc",
            ObjectiveCategory::SecurityHardening,
            risk,
            roi,
            orientation,
            urgency,
            10.0,
        )
    }

    #[test]
    fn high_risk_scores_higher() {
        let high = ScoredObjective::from_objective(&obj(0.9, 0.8, 0.7, 0.8));
        let low = ScoredObjective::from_objective(&obj(0.1, 0.5, 0.3, 0.2));
        assert!(high.score > low.score);
    }

    #[test]
    fn ranking_sorted_descending() {
        let objectives = vec![
            obj(0.3, 0.4, 0.3, 0.2),
            obj(0.9, 0.9, 0.8, 0.9),
            obj(0.5, 0.6, 0.5, 0.5),
        ];
        let ranked = rank_objectives(&objectives);
        for w in ranked.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn below_threshold_filtered() {
        let objectives = vec![obj(0.0, 0.0, 0.0, 0.0)]; // score = 0
        let ranked = rank_objectives(&objectives);
        assert!(ranked.is_empty());
    }
}
