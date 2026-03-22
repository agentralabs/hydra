//! SettlementRecord — one fully settled task.
//! Immutable once written. Constitutional.
//! Every action Hydra takes settles into one of these.

use crate::cost::CostItem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// What was produced by this task — the net outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    /// Task completed successfully.
    Success { description: String },
    /// Task completed via alternative approach.
    SuccessViaAlternative { approach: String },
    /// Task was hard denied.
    HardDenied { evidence: String },
    /// Task was suspended (paused, not failed).
    Suspended { condition: String },
}

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Success { .. } => "success",
            Self::SuccessViaAlternative { .. } => "success-via-alternative",
            Self::HardDenied { .. } => "hard-denied",
            Self::Suspended { .. } => "suspended",
        }
    }
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Success { .. } | Self::SuccessViaAlternative { .. }
        )
    }
}

/// One settled task record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRecord {
    pub id: String,
    pub task_id: String,
    pub action_id: String,
    pub domain: String,
    pub intent: String,
    pub outcome: Outcome,
    pub costs: Vec<CostItem>,
    pub total_cost: f64,
    pub duration_ms: u64,
    pub attempt_count: u32,
    pub integrity_hash: String,
    pub settled_at: chrono::DateTime<chrono::Utc>,
}

impl SettlementRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_id: impl Into<String>,
        action_id: impl Into<String>,
        domain: impl Into<String>,
        intent: impl Into<String>,
        outcome: Outcome,
        costs: Vec<CostItem>,
        duration_ms: u64,
        attempt_count: u32,
    ) -> Self {
        let task_id_s = task_id.into();
        let action_id_s = action_id.into();
        let domain_s = domain.into();
        let intent_s = intent.into();
        let total_cost = costs.iter().map(|c| c.total).sum();
        let now = chrono::Utc::now();

        let hash = Self::compute_hash(&task_id_s, &action_id_s, &domain_s, total_cost, &now);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task_id_s,
            action_id: action_id_s,
            domain: domain_s,
            intent: intent_s,
            outcome,
            costs,
            total_cost,
            duration_ms,
            attempt_count,
            integrity_hash: hash,
            settled_at: now,
        }
    }

    fn compute_hash(
        task_id: &str,
        action_id: &str,
        domain: &str,
        cost: f64,
        at: &chrono::DateTime<chrono::Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(task_id.as_bytes());
        h.update(action_id.as_bytes());
        h.update(domain.as_bytes());
        h.update(cost.to_bits().to_le_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }

    /// Total overhead cost (rerouting + redteam).
    pub fn overhead_cost(&self) -> f64 {
        self.costs
            .iter()
            .filter(|c| c.class.is_overhead())
            .map(|c| c.total)
            .sum()
    }

    /// Efficiency: success / total cost. Higher = more efficient.
    pub fn efficiency_ratio(&self) -> f64 {
        if self.total_cost < 1e-10 {
            return 1.0;
        }
        if self.outcome.is_success() {
            1.0 / self.total_cost
        } else {
            0.0
        }
    }

    /// Cost breakdown by class label.
    pub fn cost_by_class(&self) -> Vec<(String, f64)> {
        let mut breakdown: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for item in &self.costs {
            *breakdown.entry(item.class.label()).or_insert(0.0) += item.total;
        }
        let mut result: Vec<(String, f64)> = breakdown.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::CostClass;

    fn make_record(domain: &str, success: bool) -> SettlementRecord {
        let costs = vec![
            CostItem::new(CostClass::DirectExecution, 2000, 10.0, 3000),
            CostItem::new(
                CostClass::SisterCall {
                    sister_name: "AgenticMemory".into(),
                },
                500,
                2.0,
                500,
            ),
        ];
        SettlementRecord::new(
            "task-1",
            "deploy.staging",
            domain,
            "deploy to staging",
            if success {
                Outcome::Success {
                    description: "deployed".into(),
                }
            } else {
                Outcome::HardDenied {
                    evidence: "auth rejected".into(),
                }
            },
            costs,
            3500,
            1,
        )
    }

    #[test]
    fn record_integrity_hash_valid() {
        let r = make_record("engineering", true);
        assert!(r.verify_integrity());
        assert_eq!(r.integrity_hash.len(), 64);
    }

    #[test]
    fn total_cost_sum_of_items() {
        let r = make_record("engineering", true);
        let expected: f64 = r.costs.iter().map(|c| c.total).sum();
        assert!((r.total_cost - expected).abs() < 1e-10);
    }

    #[test]
    fn efficiency_zero_for_denied() {
        let r = make_record("engineering", false);
        assert_eq!(r.efficiency_ratio(), 0.0);
    }

    #[test]
    fn efficiency_positive_for_success() {
        let r = make_record("engineering", true);
        assert!(r.efficiency_ratio() > 0.0);
    }

    #[test]
    fn cost_breakdown_by_class() {
        let r = make_record("engineering", true);
        let breakdown = r.cost_by_class();
        assert!(!breakdown.is_empty());
    }
}
