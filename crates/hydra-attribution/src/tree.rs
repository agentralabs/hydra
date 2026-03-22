//! AttributionTree — full causal trace for one settlement record.

use crate::{
    cause::{infer_factors, CausalFactor},
    cost::CostItem,
};
use serde::{Deserialize, Serialize};

/// The attribution tree for one settlement record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionTree {
    pub id: String,
    pub record_id: String,
    pub task_id: String,
    pub action_id: String,
    pub domain: String,
    pub total_cost: f64,
    pub factors: Vec<CausalFactor>,
    pub avoidable_cost: f64,
    pub avoidable_fraction: f64,
    pub one_time_cost: f64,
    pub narrative: String,
    pub built_at: chrono::DateTime<chrono::Utc>,
}

impl AttributionTree {
    /// Build an attribution tree from settlement record fields and
    /// pre-converted attribution cost items.
    pub fn from_record(
        record_id: &str,
        task_id: &str,
        action_id: &str,
        domain: &str,
        costs: &[CostItem],
        total_cost: f64,
        context: &str,
    ) -> Self {
        let factors = infer_factors(costs, total_cost, context);

        let avoidable_cost: f64 = factors
            .iter()
            .filter(|f| f.is_avoidable())
            .map(|f| f.cost_fraction * total_cost)
            .sum();

        let one_time_cost: f64 = factors
            .iter()
            .filter(|f| f.is_one_time())
            .map(|f| f.cost_fraction * total_cost)
            .sum();

        let avoidable_fraction = if total_cost > 1e-10 {
            avoidable_cost / total_cost
        } else {
            0.0
        };

        let narrative =
            build_narrative(action_id, domain, total_cost, &factors, avoidable_fraction);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            record_id: record_id.to_string(),
            task_id: task_id.to_string(),
            action_id: action_id.to_string(),
            domain: domain.to_string(),
            total_cost,
            factors,
            avoidable_cost,
            avoidable_fraction,
            one_time_cost,
            narrative,
            built_at: chrono::Utc::now(),
        }
    }

    pub fn has_avoidable_cost(&self) -> bool {
        self.avoidable_fraction >= crate::constants::REROUTING_COST_FLAG_THRESHOLD
    }

    pub fn primary_cause(&self) -> Option<&CausalFactor> {
        self.factors.first()
    }

    pub fn avoidable_recommendations(&self) -> Vec<&str> {
        self.factors
            .iter()
            .filter(|f| f.is_avoidable())
            .filter_map(|f| f.recommendation.as_deref())
            .collect()
    }
}

fn build_narrative(
    action_id: &str,
    domain: &str,
    total_cost: f64,
    factors: &[CausalFactor],
    avoidable: f64,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Attribution for '{}' ({}) — total cost: {:.2}",
        action_id, domain, total_cost,
    ));

    for f in factors {
        let avoidable_tag = if f.is_avoidable() {
            " [avoidable]"
        } else if f.is_one_time() {
            " [one-time]"
        } else {
            ""
        };
        lines.push(format!(
            "  → {:.0}% {}{}",
            f.cost_fraction * 100.0,
            f.description,
            avoidable_tag
        ));
        if let Some(rec) = &f.recommendation {
            lines.push(format!("    Recommendation: {}", rec));
        }
    }

    if avoidable > 0.0 {
        lines.push(format!(
            "  Avoidable: {:.0}% of total cost could be reduced.",
            avoidable * 100.0
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostClass, CostItem};

    fn make_costs() -> (Vec<CostItem>, f64) {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 2000, 3.0, 3000),
            CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.5, 500),
            CostItem::new(
                CostClass::SisterCallCost {
                    sister_name: "AgenticMemory".into(),
                },
                300,
                0.3,
                300,
            ),
        ];
        let total: f64 = costs.iter().map(|c| c.amount).sum();
        (costs, total)
    }

    #[test]
    fn tree_built_from_record() {
        let (costs, total) = make_costs();
        let tree = AttributionTree::from_record(
            "rec-1",
            "task-1",
            "deploy.staging",
            "engineering",
            &costs,
            total,
            "deploy service with concurrent lock conflict",
        );
        assert!(!tree.factors.is_empty());
        assert_eq!(tree.action_id, "deploy.staging");
        assert!(!tree.narrative.is_empty());
    }

    #[test]
    fn avoidable_cost_detected() {
        let (costs, total) = make_costs();
        let tree = AttributionTree::from_record(
            "rec-1",
            "task-1",
            "deploy.staging",
            "engineering",
            &costs,
            total,
            "deploy service with concurrent lock conflict",
        );
        assert!(tree.avoidable_cost >= 0.0);
        let has_avoidable = tree.factors.iter().any(|f| f.is_avoidable());
        assert!(has_avoidable);
    }

    #[test]
    fn primary_cause_is_largest() {
        let (costs, total) = make_costs();
        let tree = AttributionTree::from_record(
            "rec-1",
            "task-1",
            "deploy.staging",
            "engineering",
            &costs,
            total,
            "deploy service",
        );
        if let (Some(primary), Some(second)) = (tree.factors.first(), tree.factors.get(1)) {
            assert!(primary.cost_fraction >= second.cost_fraction);
        }
    }

    #[test]
    fn narrative_non_empty() {
        let (costs, total) = make_costs();
        let tree = AttributionTree::from_record(
            "rec-1",
            "task-1",
            "deploy.staging",
            "engineering",
            &costs,
            total,
            "deploy service with concurrent lock conflict",
        );
        assert!(tree.narrative.contains("Attribution for"));
        assert!(tree.narrative.contains("deploy.staging"));
    }
}
