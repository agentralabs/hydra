//! AvoidabilityAssessor — aggregate avoidable cost analysis across a period.

use crate::tree::AttributionTree;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregated avoidability analysis for a set of attribution trees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvoidabilityReport {
    pub total_cost: f64,
    pub avoidable_cost: f64,
    pub avoidable_fraction: f64,
    pub one_time_cost: f64,
    /// Top avoidable causes and their total cost.
    pub top_avoidable: Vec<(String, f64)>,
    /// Recommendations sorted by potential savings.
    pub recommendations: Vec<String>,
    pub alert: bool,
}

impl AvoidabilityReport {
    pub fn from_trees(trees: &[&AttributionTree]) -> Self {
        if trees.is_empty() {
            return Self::empty();
        }

        let total_cost: f64 = trees.iter().map(|t| t.total_cost).sum();
        let avoidable_cost: f64 = trees.iter().map(|t| t.avoidable_cost).sum();
        let one_time_cost: f64 = trees.iter().map(|t| t.one_time_cost).sum();

        let avoidable_fraction = if total_cost > 1e-10 {
            avoidable_cost / total_cost
        } else {
            0.0
        };

        // Aggregate avoidable causes
        let mut cause_costs: HashMap<String, f64> = HashMap::new();
        for tree in trees {
            for factor in &tree.factors {
                if factor.is_avoidable() {
                    let cost = factor.cost_fraction * tree.total_cost;
                    *cause_costs.entry(factor.factor_type.label()).or_insert(0.0) += cost;
                }
            }
        }

        let mut top_avoidable: Vec<(String, f64)> = cause_costs.into_iter().collect();
        top_avoidable.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Collect unique recommendations
        let mut seen = std::collections::HashSet::new();
        let recommendations: Vec<String> = trees
            .iter()
            .flat_map(|t| t.avoidable_recommendations())
            .filter(|r| seen.insert(r.to_string()))
            .map(String::from)
            .collect();

        let alert = avoidable_fraction >= crate::constants::AVOIDABLE_COST_ALERT_THRESHOLD;

        Self {
            total_cost,
            avoidable_cost,
            avoidable_fraction,
            one_time_cost,
            top_avoidable,
            recommendations,
            alert,
        }
    }

    pub fn empty() -> Self {
        Self {
            total_cost: 0.0,
            avoidable_cost: 0.0,
            avoidable_fraction: 0.0,
            one_time_cost: 0.0,
            top_avoidable: vec![],
            recommendations: vec![],
            alert: false,
        }
    }

    pub fn brief(&self) -> String {
        format!(
            "Avoidability: {:.0}% of cost avoidable ({:.1} of {:.1} total){}",
            self.avoidable_fraction * 100.0,
            self.avoidable_cost,
            self.total_cost,
            if self.alert { " ⚠ ALERT" } else { "" },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostClass, CostItem};
    use crate::tree::AttributionTree;

    fn tree_with_rerouting(task_id: &str) -> AttributionTree {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 1000, 2.0, 2000),
            CostItem::new(CostClass::ReroutingOverhead { attempts: 2 }, 0, 0.5, 500),
        ];
        let total: f64 = costs.iter().map(|c| c.amount).sum();
        AttributionTree::from_record(
            &format!("rec-{}", task_id),
            task_id,
            "a.id",
            "engineering",
            &costs,
            total,
            "deploy with concurrent lock",
        )
    }

    #[test]
    fn report_aggregates_avoidable() {
        let t1 = tree_with_rerouting("task-1");
        let t2 = tree_with_rerouting("task-2");
        let report = AvoidabilityReport::from_trees(&[&t1, &t2]);
        assert!(report.avoidable_cost > 0.0);
        assert!(report.total_cost > 0.0);
    }

    #[test]
    fn alert_triggered_above_threshold() {
        let mut trees = Vec::new();
        for i in 0..5 {
            trees.push(tree_with_rerouting(&format!("t{}", i)));
        }
        let refs: Vec<&AttributionTree> = trees.iter().collect();
        let report = AvoidabilityReport::from_trees(&refs);
        if report.avoidable_fraction >= crate::constants::AVOIDABLE_COST_ALERT_THRESHOLD {
            assert!(report.alert);
        }
    }

    #[test]
    fn empty_trees_no_alert() {
        let report = AvoidabilityReport::from_trees(&[]);
        assert!(!report.alert);
        assert_eq!(report.total_cost, 0.0);
    }
}
