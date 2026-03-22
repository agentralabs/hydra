//! SettlementEngine — the cost accounting coordinator.

use crate::{
    cost::{CostClass, CostItem},
    errors::SettlementError,
    ledger::SettlementLedger,
    period::SettlementPeriod,
    record::{Outcome, SettlementRecord},
};
use hydra_executor::TaskRecord;

/// The settlement engine.
pub struct SettlementEngine {
    pub ledger: SettlementLedger,
}

impl SettlementEngine {
    pub fn new() -> Self {
        Self {
            ledger: SettlementLedger::new(),
        }
    }

    /// Create a settlement engine backed by SQLite persistence.
    pub fn open() -> Self {
        Self {
            ledger: SettlementLedger::open(),
        }
    }

    /// Settle a completed task from hydra-executor.
    pub fn settle_task(
        &mut self,
        task: &TaskRecord,
        domain: &str,
    ) -> Result<&SettlementRecord, SettlementError> {
        // Build cost items from the task record
        let mut costs = Vec::new();

        // Direct execution cost (estimate from attempts and duration)
        let total_duration_ms: u64 = task.attempts.iter().map(|a| a.duration_ms).sum();
        let estimated_tokens: u64 = task.attempts.len() as u64 * 500;

        costs.push(CostItem::new(
            CostClass::DirectExecution,
            estimated_tokens,
            task.attempts.len() as f64 * 5.0,
            total_duration_ms,
        ));

        // Rerouting overhead (if multiple attempts)
        let attempt_count = task.attempts.len() as u32;
        if attempt_count > 1 {
            costs.push(
                CostItem::new(
                    CostClass::ReroutingOverhead {
                        attempts: attempt_count - 1,
                    },
                    0,
                    0.0,
                    0,
                )
                .with_rerouting_overhead(attempt_count - 1),
            );
        }

        // Map task state to outcome
        let outcome = match &task.state {
            hydra_executor::TaskState::Complete { .. } => Outcome::Success {
                description: format!("Completed: {}", task.action_id),
            },
            hydra_executor::TaskState::HardDenied { evidence, .. } => Outcome::HardDenied {
                evidence: evidence.clone(),
            },
            _ => Outcome::Suspended {
                condition: "task in non-terminal state".into(),
            },
        };

        let record = SettlementRecord::new(
            &task.id,
            &task.action_id,
            domain,
            &task.intent,
            outcome,
            costs,
            total_duration_ms,
            attempt_count,
        );

        self.ledger.settle(record)?;
        // Safe: we just inserted a record with this task_id, so get_by_task
        // will always find it. Using expect with a clear message.
        Ok(self
            .ledger
            .get_by_task(&task.id)
            .expect("record was just inserted"))
    }

    /// Settle a skill action (from skills system).
    #[allow(clippy::too_many_arguments)]
    pub fn settle_skill_action(
        &mut self,
        skill_name: &str,
        action_id: &str,
        domain: &str,
        intent: &str,
        tokens: u64,
        duration_ms: u64,
        success: bool,
    ) -> Result<(), SettlementError> {
        let costs = vec![CostItem::new(
            CostClass::SkillAction {
                skill_name: skill_name.to_string(),
                action_id: action_id.to_string(),
            },
            tokens,
            5.0,
            duration_ms,
        )];
        let outcome = if success {
            Outcome::Success {
                description: format!("Skill action {} completed", action_id),
            }
        } else {
            Outcome::HardDenied {
                evidence: "skill action denied".into(),
            }
        };
        let record = SettlementRecord::new(
            uuid::Uuid::new_v4().to_string(),
            action_id,
            domain,
            intent,
            outcome,
            costs,
            duration_ms,
            1,
        );
        self.ledger.settle(record)
    }

    /// Get settlement period for a time window.
    pub fn period(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> SettlementPeriod {
        self.ledger.period(start, end)
    }

    /// Monthly summary — the intelligence brief entry point.
    pub fn monthly_brief(&self) -> String {
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::days(30);
        let p = self.period(start, now);

        let top_domain = p
            .top_domain()
            .map(|(d, c)| format!("{} ({:.1})", d, c))
            .unwrap_or_else(|| "none".into());

        format!(
            "Settlement brief (30d): {} tasks | {:.1} total cost | \
             {:.0}% success | top domain: {} | efficiency: {:.3}",
            p.record_count,
            p.total_cost,
            (p.success_count as f64 / p.record_count.max(1) as f64) * 100.0,
            top_domain,
            p.efficiency,
        )
    }

    pub fn record_count(&self) -> usize {
        self.ledger.count()
    }

    pub fn lifetime_cost(&self) -> f64 {
        self.ledger.lifetime_cost()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "settlement: records={} lifetime_cost={:.1}",
            self.record_count(),
            self.lifetime_cost(),
        )
    }
}

impl Default for SettlementEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_executor::{ExecutionEngine, ExecutionRequest, ExecutorType, RegisteredAction};
    use std::collections::HashMap;

    fn run_task(action_id: &str) -> TaskRecord {
        let mut engine = ExecutionEngine::new();
        engine.registry_mut().register_skill_actions(
            "test",
            vec![RegisteredAction {
                id: action_id.to_string(),
                skill: "test".into(),
                description: "test".into(),
                verb: "testing".into(),
                executor: ExecutorType::Internal {
                    handler: "succeed".into(),
                },
                reversible: false,
                estimated_ms: 100,
                input_params: vec![],
            }],
        );
        engine
            .execute(ExecutionRequest::new(
                action_id,
                "test intent",
                HashMap::new(),
            ))
            .expect("execution should succeed")
    }

    #[test]
    fn settle_executor_task() {
        let mut engine = SettlementEngine::new();
        let task = run_task("deploy.staging");
        engine
            .settle_task(&task, "engineering")
            .expect("settlement should succeed");
        assert_eq!(engine.record_count(), 1);
        assert!(engine.lifetime_cost() > 0.0);
    }

    #[test]
    fn settle_skill_action() {
        let mut engine = SettlementEngine::new();
        engine
            .settle_skill_action(
                "agentra-settlement",
                "settlement.execute",
                "fintech",
                "execute settlement batch",
                1500,
                2000,
                true,
            )
            .expect("skill settlement should succeed");
        assert_eq!(engine.record_count(), 1);
    }

    #[test]
    fn period_aggregation() {
        let mut engine = SettlementEngine::new();
        for i in 0..5 {
            engine
                .settle_skill_action(
                    "test-skill",
                    &format!("action.{}", i),
                    "engineering",
                    "intent",
                    1000,
                    1000,
                    true,
                )
                .expect("skill settlement should succeed");
        }
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let p = engine.period(start, now);
        assert_eq!(p.record_count, 5);
        assert_eq!(p.success_count, 5);
    }

    #[test]
    fn monthly_brief_format() {
        let engine = SettlementEngine::new();
        let brief = engine.monthly_brief();
        assert!(brief.contains("Settlement brief"));
        assert!(brief.contains("tasks"));
        assert!(brief.contains("efficiency"));
    }

    #[test]
    fn summary_format() {
        let engine = SettlementEngine::new();
        let s = engine.summary();
        assert!(s.contains("settlement:"));
        assert!(s.contains("records="));
    }
}
