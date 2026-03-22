//! AuditEngine — the accountability coordinator.
//! Receives task records from executor -> builds trace -> generates
//! narrative -> persists record -> queryable forever.

use crate::{
    errors::AuditError,
    event::{EventKind, TraceEvent},
    narrative::NarrativeBuilder,
    record::{AuditQuery, AuditRecord, AuditStore},
    trace::ExecutionTrace,
};
use hydra_constitution::{ConstitutionChecker, LawCheckContext};
use hydra_executor::TaskRecord;
use std::collections::HashMap;

/// The audit engine.
pub struct AuditEngine {
    builder: NarrativeBuilder,
    /// The append-only audit store.
    pub store: AuditStore,
    traces: HashMap<String, ExecutionTrace>,
    checker: ConstitutionChecker,
}

impl AuditEngine {
    /// Create a new audit engine (in-memory only).
    pub fn new() -> Self {
        Self {
            builder: NarrativeBuilder::new(),
            store: AuditStore::new(),
            traces: HashMap::new(),
            checker: ConstitutionChecker::new(),
        }
    }

    /// Create an audit engine backed by SQLite persistence.
    pub fn open() -> Self {
        Self {
            builder: NarrativeBuilder::new(),
            store: AuditStore::open(),
            traces: HashMap::new(),
            checker: ConstitutionChecker::new(),
        }
    }

    /// Audit a completed task record from hydra-executor.
    pub fn audit_task(
        &mut self,
        task: &TaskRecord,
    ) -> Result<String, AuditError> {
        let mut trace = ExecutionTrace::new(&task.id, &task.action_id);

        // Build trace from task record
        trace.add_event(TraceEvent::new(
            &task.id,
            EventKind::TaskStarted {
                intent: task.intent.clone(),
            },
            "genesis",
            0,
        ));

        for attempt in &task.attempts {
            trace.add_event(TraceEvent::new(
                &task.id,
                EventKind::ApproachAttempted {
                    approach: attempt.approach.label().to_string(),
                },
                &attempt.receipt_id,
                attempt.duration_ms,
            ));

            if let Some(obstacle) = &attempt.obstacle {
                trace.add_event(TraceEvent::new(
                    &task.id,
                    EventKind::ObstacleEncountered {
                        obstacle: obstacle.clone(),
                    },
                    &attempt.receipt_id,
                    0,
                ));
            }

            if let Some(next) = &attempt.next_approach {
                trace.add_event(TraceEvent::new(
                    &task.id,
                    EventKind::Rerouting {
                        from: attempt.approach.label().to_string(),
                        to: next.label().to_string(),
                    },
                    &attempt.receipt_id,
                    0,
                ));
            }
        }

        // Terminal event
        match &task.state {
            hydra_executor::TaskState::Complete { receipt_id } => {
                trace.add_event(TraceEvent::new(
                    &task.id,
                    EventKind::TaskCompleted {
                        duration_total_ms: trace.total_duration_ms(),
                    },
                    receipt_id,
                    0,
                ));
            }
            hydra_executor::TaskState::HardDenied {
                evidence,
                receipt_id,
            } => {
                trace.add_event(TraceEvent::new(
                    &task.id,
                    EventKind::TaskHardDenied {
                        evidence: evidence.clone(),
                    },
                    receipt_id,
                    0,
                ));
            }
            _ => {}
        }

        // Collect receipt IDs
        let receipt_ids: Vec<String> =
            task.attempts.iter().map(|a| a.receipt_id.clone()).collect();

        // Build narrative
        let narrative = self.builder.build(&trace)?;

        // Constitutional check: Law 1 (Receipt Immutability)
        let ctx = LawCheckContext::new(&task.id, "receipt.write")
            .with_meta("action_id", &task.action_id);
        if let Err(e) = self.checker.check_strict(&ctx) {
            eprintln!("hydra: audit write BLOCKED by constitution: {e}");
            return Err(AuditError::ConstitutionalViolation {
                reason: format!("{e}"),
            });
        }

        // Create and store audit record
        let record = AuditRecord::from_narrative(&narrative, receipt_ids);
        let summary = record.summary.clone();
        self.store.append(record);
        self.traces.insert(task.id.clone(), trace);

        Ok(summary)
    }

    /// Build a trace manually (for non-executor audits).
    pub fn audit_manual(
        &mut self,
        task_id: &str,
        action_id: &str,
        events: Vec<(EventKind, &str, u64)>,
    ) -> Result<String, AuditError> {
        // Constitutional check: Law 1 (Receipt Immutability)
        let ctx = LawCheckContext::new(task_id, "receipt.write")
            .with_meta("action_id", action_id);
        if let Err(e) = self.checker.check_strict(&ctx) {
            eprintln!("hydra: audit write BLOCKED by constitution: {e}");
            return Err(AuditError::ConstitutionalViolation {
                reason: format!("{e}"),
            });
        }

        let mut trace = ExecutionTrace::new(task_id, action_id);
        for (kind, receipt_id, duration_ms) in events {
            trace.add_event(TraceEvent::new(task_id, kind, receipt_id, duration_ms));
        }
        let narrative = self.builder.build(&trace)?;
        let record = AuditRecord::from_narrative(&narrative, vec![]);
        let summary = record.summary.clone();
        self.store.append(record);
        Ok(summary)
    }

    /// Query audit records.
    pub fn query(&self, q: &AuditQuery) -> Vec<&AuditRecord> {
        self.store.query(q)
    }

    /// Total number of records.
    pub fn record_count(&self) -> usize {
        self.store.count()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        let successful = self
            .store
            .query(&AuditQuery {
                outcome: Some("completed".into()),
                ..Default::default()
            })
            .len();
        format!(
            "audit: records={} successful={} traces={}",
            self.record_count(),
            successful,
            self.traces.len(),
        )
    }
}

impl Default for AuditEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_executor::{
        ExecutionEngine, ExecutionRequest, ExecutorType,
        RegisteredAction,
    };
    use std::collections::HashMap;

    fn setup_engine_with_action(id: &str) -> ExecutionEngine {
        let mut engine = ExecutionEngine::new();
        engine
            .registry_mut()
            .register_skill_actions("test", vec![RegisteredAction {
                id: id.to_string(),
                skill: "test".into(),
                description: "test".into(),
                verb: "testing".into(),
                executor: ExecutorType::Internal {
                    handler: "succeed".into(),
                },
                reversible: false,
                estimated_ms: 100,
                input_params: vec![],
            }]);
        engine
    }

    #[test]
    fn audit_completed_task() {
        let mut exec =
            setup_engine_with_action("deploy.staging");
        let req = ExecutionRequest::new(
            "deploy.staging",
            "deploy",
            HashMap::new(),
        );
        let task = exec.execute(req).expect("execute");

        let mut audit = AuditEngine::new();
        let summary = audit.audit_task(&task).expect("audit");

        assert!(!summary.is_empty());
        assert!(summary.contains("deploy.staging"));
        assert_eq!(audit.record_count(), 1);
    }

    #[test]
    fn audit_record_integrity() {
        let mut exec = setup_engine_with_action("test.action");
        let req = ExecutionRequest::new(
            "test.action",
            "test intent",
            HashMap::new(),
        );
        let task = exec.execute(req).expect("execute");

        let mut audit = AuditEngine::new();
        audit.audit_task(&task).expect("audit");

        let record =
            audit.store.query(&AuditQuery::default())[0];
        assert!(record.verify_integrity());
    }

    #[test]
    fn manual_audit_with_obstacles() {
        let mut audit = AuditEngine::new();
        let summary = audit
            .audit_manual(
                "task-manual",
                "deploy.prod",
                vec![
                    (
                        EventKind::TaskStarted {
                            intent: "deploy to production".into(),
                        },
                        "r1",
                        0,
                    ),
                    (
                        EventKind::ApproachAttempted {
                            approach: "direct".into(),
                        },
                        "r2",
                        100,
                    ),
                    (
                        EventKind::ObstacleEncountered {
                            obstacle: "auth certificate expired".into(),
                        },
                        "r3",
                        50,
                    ),
                    (
                        EventKind::Rerouting {
                            from: "direct".into(),
                            to: "alternative".into(),
                        },
                        "r4",
                        10,
                    ),
                    (
                        EventKind::ApproachAttempted {
                            approach: "alternative".into(),
                        },
                        "r5",
                        200,
                    ),
                    (
                        EventKind::ApproachSucceeded {
                            approach: "alternative".into(),
                        },
                        "r6",
                        5,
                    ),
                    (
                        EventKind::TaskCompleted {
                            duration_total_ms: 365,
                        },
                        "r7",
                        0,
                    ),
                ],
            )
            .expect("audit");

        assert!(summary.contains("deploy.prod"));
        let record =
            audit.store.get_by_task("task-manual").expect("record");
        assert_eq!(record.attempt_count, 2);
        assert_eq!(record.obstacle_count, 1);
        assert!(record.is_successful());
    }

    #[test]
    fn query_by_action_id() {
        let mut audit = AuditEngine::new();
        audit
            .audit_manual("t1", "action.a", vec![
                (
                    EventKind::TaskStarted {
                        intent: "i".into(),
                    },
                    "r",
                    0,
                ),
                (
                    EventKind::TaskCompleted {
                        duration_total_ms: 100,
                    },
                    "r2",
                    0,
                ),
            ])
            .expect("audit");
        audit
            .audit_manual("t2", "action.b", vec![
                (
                    EventKind::TaskStarted {
                        intent: "i".into(),
                    },
                    "r",
                    0,
                ),
                (
                    EventKind::TaskCompleted {
                        duration_total_ms: 100,
                    },
                    "r2",
                    0,
                ),
            ])
            .expect("audit");

        let results = audit.query(&AuditQuery {
            action_id: Some("action.a".into()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_id, "action.a");
    }
}
