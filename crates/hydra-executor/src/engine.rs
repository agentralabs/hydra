//! ExecutionEngine — the relentless task machine.
//! FAILED does not exist. Every obstacle is navigational.

use crate::{
    constants::*,
    errors::ExecutorError,
    receipt::{ExecutionReceipt, ReceiptLedger, ReceiptOutcome},
    registry::ActionRegistry,
    runner::ActionRunner,
    task::{ApproachType, AttemptRecord, TaskRecord, TaskState},
};
use std::collections::HashMap;

/// Execution request — what to run and with what parameters.
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub action_id: String,
    pub intent: String,
    pub params: HashMap<String, String>,
}

impl ExecutionRequest {
    pub fn new(
        action_id: impl Into<String>,
        intent: impl Into<String>,
        params: HashMap<String, String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            intent: intent.into(),
            params,
        }
    }
}

/// The execution engine — runs requests through the relentless machine.
pub struct ExecutionEngine {
    registry: ActionRegistry,
    runner: ActionRunner,
    pub ledger: ReceiptLedger,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            registry: ActionRegistry::new(),
            runner: ActionRunner::new(),
            ledger: ReceiptLedger::new(),
        }
    }

    pub fn registry_mut(&mut self) -> &mut ActionRegistry {
        &mut self.registry
    }

    /// Execute a request through the relentless task machine.
    pub fn execute(
        &mut self,
        request: ExecutionRequest,
    ) -> Result<TaskRecord, ExecutorError> {
        // Look up the action
        let action = self
            .registry
            .get(&request.action_id)
            .ok_or_else(|| ExecutorError::ActionNotFound {
                id: request.action_id.clone(),
            })?
            .clone();

        // Create task record
        let mut task =
            TaskRecord::new(&request.action_id, &request.intent);

        // Receipt BEFORE execution starts (write-ahead — constitutional)
        let start_receipt = ExecutionReceipt::for_start(
            &task.id,
            &request.action_id,
            &request.intent,
            ApproachType::DirectExecution.label(),
        );
        self.ledger.record(start_receipt.clone());

        // Approach escalation loop
        let approaches = ApproachType::all_in_order();
        let mut attempt_num = 0u32;

        for approach in &approaches {
            if attempt_num >= MAX_APPROACH_ATTEMPTS {
                break;
            }
            attempt_num += 1;

            task.transition(TaskState::Active {
                approach: approach.clone(),
            });

            // Run the action
            let result = self.runner.run(&action, &request.params);
            let duration_ms = result.duration_ms;

            if result.success {
                // Receipt the completion
                let mut complete_receipt = ExecutionReceipt::for_start(
                    &task.id,
                    &request.action_id,
                    &request.intent,
                    approach.label(),
                );
                complete_receipt.outcome = ReceiptOutcome::Succeeded;
                let receipt_id = complete_receipt.id.clone();
                self.ledger.record(complete_receipt);

                task.add_attempt(AttemptRecord {
                    attempt_number: attempt_num,
                    approach: approach.clone(),
                    obstacle: None,
                    resolution: Some(result.output.clone()),
                    next_approach: None,
                    duration_ms,
                    receipt_id: receipt_id.clone(),
                });

                task.transition(TaskState::Complete {
                    receipt_id,
                });

                return Ok(task);
            }

            // Blocked — record and try next approach
            let obstacle = result
                .obstacle
                .unwrap_or_else(|| "unknown obstacle".into());

            task.add_attempt(AttemptRecord {
                attempt_number: attempt_num,
                approach: approach.clone(),
                obstacle: Some(obstacle.clone()),
                resolution: None,
                next_approach: approaches
                    .get(attempt_num as usize)
                    .cloned(),
                duration_ms,
                receipt_id: start_receipt.id.clone(),
            });

            task.transition(TaskState::Blocked {
                reason: obstacle.clone(),
                approach: approach.clone(),
            });

            // Check for hard stop conditions
            if obstacle.contains(HARD_DENIED_AUTH)
                || obstacle.contains(HARD_DENIED_PRINCIPAL)
                || obstacle.contains(HARD_DENIED_CONSTITUTIONAL)
            {
                let mut denied_receipt = ExecutionReceipt::for_start(
                    &task.id,
                    &request.action_id,
                    &request.intent,
                    "hard-denied",
                );
                denied_receipt.outcome = ReceiptOutcome::HardDenied {
                    evidence: obstacle.clone(),
                };
                let denied_id = denied_receipt.id.clone();
                self.ledger.record(denied_receipt);
                task.transition(TaskState::HardDenied {
                    evidence: obstacle,
                    receipt_id: denied_id,
                });
                return Ok(task);
            }

            // Not a hard stop — reroute and continue
            if let Some(next) =
                approaches.get(attempt_num as usize)
            {
                task.transition(TaskState::Rerouting {
                    attempt: attempt_num,
                    next_approach: next.clone(),
                });
            }
        }

        // Exhausted all approaches without hard-denial evidence
        Err(ExecutorError::ApproachesExhausted {
            attempts: attempt_num,
        })
    }

    pub fn receipt_count(&self) -> usize {
        self.ledger.count()
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ExecutorType, RegisteredAction};

    fn register_action(
        engine: &mut ExecutionEngine,
        id: &str,
        succeeds: bool,
    ) {
        let executor = if succeeds {
            ExecutorType::Internal {
                handler: "succeed".to_string(),
            }
        } else {
            ExecutorType::Shell {
                command_template: "cmd {unresolved_param}".to_string(),
            }
        };
        engine.registry_mut().register_skill_actions(
            "test",
            vec![RegisteredAction {
                id: id.to_string(),
                skill: "test".into(),
                description: "test action".into(),
                verb: "testing".into(),
                executor,
                reversible: false,
                estimated_ms: 10,
                input_params: vec![],
            }],
        );
    }

    #[test]
    fn successful_execution_completes() {
        let mut engine = ExecutionEngine::new();
        register_action(&mut engine, "test.succeed", true);
        let req = ExecutionRequest::new(
            "test.succeed",
            "test intent",
            HashMap::new(),
        );
        let result = engine.execute(req);
        let task = result.expect("should succeed");
        assert!(matches!(task.state, TaskState::Complete { .. }));
    }

    #[test]
    fn execution_receipted_before_completion() {
        let mut engine = ExecutionEngine::new();
        register_action(&mut engine, "test.action", true);
        let req =
            ExecutionRequest::new("test.action", "test", HashMap::new());
        let _ = engine.execute(req);
        assert!(engine.receipt_count() >= 1);
    }

    #[test]
    fn unknown_action_returns_error() {
        let mut engine = ExecutionEngine::new();
        let req = ExecutionRequest::new(
            "nonexistent.action",
            "test",
            HashMap::new(),
        );
        let result = engine.execute(req);
        assert!(matches!(
            result,
            Err(ExecutorError::ActionNotFound { .. })
        ));
    }

    #[test]
    fn failed_state_never_produced() {
        let mut engine = ExecutionEngine::new();
        register_action(&mut engine, "test.fail", false);
        let req =
            ExecutionRequest::new("test.fail", "test", HashMap::new());
        let result = engine.execute(req);
        if let Ok(task) = result {
            assert_ne!(task.state.label(), "failed");
        }
    }
}
