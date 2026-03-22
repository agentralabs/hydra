//! AttributionEngine — the causal cost tracing coordinator.

use crate::{
    avoidable::AvoidabilityReport,
    constants::MAX_ATTRIBUTION_RECORDS,
    cost::{CostClass, CostItem},
    errors::AttributionError,
    tree::AttributionTree,
};
use hydra_settlement::{SettlementLedger, SettlementQuery, SettlementRecord};

/// The attribution engine.
pub struct AttributionEngine {
    trees: Vec<AttributionTree>,
}

impl AttributionEngine {
    pub fn new() -> Self {
        Self { trees: Vec::new() }
    }

    /// Attribute a single settlement record.
    pub fn attribute(
        &mut self,
        record: &SettlementRecord,
    ) -> Result<&AttributionTree, AttributionError> {
        if record.costs.is_empty() {
            return Err(AttributionError::NoCostItems {
                id: record.id.clone(),
            });
        }
        if self.trees.len() >= MAX_ATTRIBUTION_RECORDS {
            return Err(AttributionError::StoreFull {
                max: MAX_ATTRIBUTION_RECORDS,
            });
        }

        let costs = convert_settlement_costs(record);
        let total_cost = record.total_cost;
        let context = &record.intent;

        let tree = AttributionTree::from_record(
            &record.id,
            &record.task_id,
            &record.action_id,
            &record.domain,
            &costs,
            total_cost,
            context,
        );
        self.trees.push(tree);
        // Safe: we just pushed an element
        Ok(self.trees.last().expect("just pushed"))
    }

    /// Attribute all records from a ledger query.
    pub fn attribute_period(
        &mut self,
        ledger: &SettlementLedger,
        query: &SettlementQuery,
    ) -> Vec<&AttributionTree> {
        let records = ledger.query(query);
        let start_idx = self.trees.len();

        for record in &records {
            let _ = self.attribute(record);
        }

        self.trees[start_idx..].iter().collect()
    }

    /// Get avoidability report for all attributed trees in a domain.
    pub fn avoidability_report(&self, domain: Option<&str>) -> AvoidabilityReport {
        let trees: Vec<&AttributionTree> = self
            .trees
            .iter()
            .filter(|t| domain.map(|d| t.domain == d).unwrap_or(true))
            .collect();
        AvoidabilityReport::from_trees(&trees)
    }

    /// Get attribution tree for a specific task.
    pub fn get_by_task(&self, task_id: &str) -> Option<&AttributionTree> {
        self.trees.iter().find(|t| t.task_id == task_id)
    }

    pub fn tree_count(&self) -> usize {
        self.trees.len()
    }

    /// Summary for TUI / intelligence brief.
    pub fn summary(&self) -> String {
        let report = self.avoidability_report(None);
        format!(
            "attribution: trees={} avoidable={:.0}%{}",
            self.tree_count(),
            report.avoidable_fraction * 100.0,
            if report.alert { " [alert]" } else { "" },
        )
    }
}

impl Default for AttributionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a settlement record's cost items into attribution cost items.
fn convert_settlement_costs(record: &SettlementRecord) -> Vec<CostItem> {
    record
        .costs
        .iter()
        .map(|si| {
            let class = match &si.class {
                hydra_settlement::CostClass::DirectExecution => CostClass::DirectExecution,
                hydra_settlement::CostClass::ReroutingOverhead { attempts } => {
                    CostClass::ReroutingOverhead {
                        attempts: *attempts,
                    }
                }
                hydra_settlement::CostClass::SisterCall { sister_name } => {
                    CostClass::SisterCallCost {
                        sister_name: sister_name.clone(),
                    }
                }
                hydra_settlement::CostClass::KnowledgeAcquisition { topic } => {
                    CostClass::KnowledgeAcquisition {
                        topic: topic.clone(),
                    }
                }
                hydra_settlement::CostClass::RedTeamAnalysis => CostClass::RedTeamCost,
                hydra_settlement::CostClass::WisdomSynthesis => CostClass::WisdomSynthesis,
                hydra_settlement::CostClass::ScheduledWork { job_name } => CostClass::SkillAction {
                    skill_name: job_name.clone(),
                },
                hydra_settlement::CostClass::SkillAction { skill_name, .. } => {
                    CostClass::SkillAction {
                        skill_name: skill_name.clone(),
                    }
                }
            };
            CostItem::new(class, si.token_cost as u64, si.total, si.time_cost as u64)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_settlement::{CostClass as SC, CostItem as SI, Outcome, SettlementRecord};

    fn make_record(task_id: &str, domain: &str, with_rerouting: bool) -> SettlementRecord {
        let mut costs = vec![SI::new(SC::DirectExecution, 2000, 10.0, 3000)];
        if with_rerouting {
            costs.push(SI::new(SC::ReroutingOverhead { attempts: 2 }, 0, 0.0, 500));
        }
        SettlementRecord::new(
            task_id,
            "deploy.staging",
            domain,
            "deploy with concurrent lock conflict",
            Outcome::Success {
                description: "done".into(),
            },
            costs,
            3500,
            if with_rerouting { 3 } else { 1 },
        )
    }

    #[test]
    fn attribute_single_record() {
        let mut engine = AttributionEngine::new();
        let record = make_record("task-1", "engineering", true);
        let tree = engine.attribute(&record).expect("should attribute");
        assert!(!tree.factors.is_empty());
        assert_eq!(engine.tree_count(), 1);
    }

    #[test]
    fn avoidability_report_generated() {
        let mut engine = AttributionEngine::new();
        for i in 0..3 {
            let r = make_record(&format!("t{}", i), "engineering", true);
            engine.attribute(&r).expect("should attribute");
        }
        let report = engine.avoidability_report(None);
        assert!(report.total_cost > 0.0);
    }

    #[test]
    fn domain_filter_in_report() {
        let mut engine = AttributionEngine::new();
        engine
            .attribute(&make_record("t1", "engineering", true))
            .expect("ok");
        engine
            .attribute(&make_record("t2", "finance", false))
            .expect("ok");

        let eng_report = engine.avoidability_report(Some("engineering"));
        let fin_report = engine.avoidability_report(Some("finance"));

        assert!(eng_report.avoidable_cost >= fin_report.avoidable_cost);
    }

    #[test]
    fn summary_format() {
        let engine = AttributionEngine::new();
        let s = engine.summary();
        assert!(s.contains("attribution:"));
        assert!(s.contains("trees="));
        assert!(s.contains("avoidable="));
    }
}
