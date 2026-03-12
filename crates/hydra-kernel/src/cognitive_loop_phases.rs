//! Cognitive loop phase execution — extracted from cognitive_loop.rs for file size.
//! Contains the run_phases method that orchestrates Perceive/Think/Decide/Act/Learn.

use hydra_core::types::{CognitivePhase, CompletionSummary};

use super::cognitive_loop::{CognitiveLoop, CycleInput, CycleOutput, CycleStatus, PhaseResult, PhaseHandler};

impl CognitiveLoop {
    pub(crate) async fn run_phases(
        &self,
        input: CycleInput,
        handler: &dyn PhaseHandler,
        start_phase: CognitivePhase,
    ) -> CycleOutput {
        let mut phases_completed = Vec::new();
        let initial_budget = self.budget.lock().remaining();

        // === PERCEIVE ===
        let perceived = if start_phase == CognitivePhase::Perceive {
            // UX: Acknowledge immediately (< 100ms)
            self.ux.send_acknowledgment("Got it!");

            match self
                .run_phase(CognitivePhase::Perceive, handler.perceive(&input))
                .await
            {
                PhaseResult::Ok(v) => {
                    phases_completed.push(CognitivePhase::Perceive);
                    v
                }
                PhaseResult::TimedOut => {
                    return self.timeout_output(phases_completed, initial_budget)
                }
                PhaseResult::Interrupted => {
                    return self.interrupt_output(phases_completed, initial_budget)
                }
                PhaseResult::BudgetExceeded => {
                    return self.budget_output(phases_completed, initial_budget)
                }
                PhaseResult::Corrupted => {
                    return self.corruption_output(phases_completed, initial_budget)
                }
                PhaseResult::Failed(e) => {
                    // Perceive uses SkipAndContinue — use defaults
                    if !handler.sisters_available() {
                        // EC-CL-004: No sisters — degrade gracefully
                        serde_json::json!({"degraded": true, "input": input.text})
                    } else {
                        return self.failed_output(e, phases_completed, initial_budget);
                    }
                }
            }
        } else {
            // Resuming from checkpoint — use checkpoint context
            input.context.clone()
        };

        // === THINK ===
        let thought = if start_phase <= CognitivePhase::Think {
            self.ux.send_progress(20.0, "Analyzing...");

            match self
                .run_phase(CognitivePhase::Think, handler.think(&perceived))
                .await
            {
                PhaseResult::Ok(v) => {
                    phases_completed.push(CognitivePhase::Think);
                    v
                }
                PhaseResult::TimedOut => {
                    return self.timeout_output(phases_completed, initial_budget)
                }
                PhaseResult::Interrupted => {
                    return self.interrupt_output(phases_completed, initial_budget)
                }
                PhaseResult::BudgetExceeded => {
                    return self.budget_output(phases_completed, initial_budget)
                }
                PhaseResult::Corrupted => {
                    return self.corruption_output(phases_completed, initial_budget)
                }
                PhaseResult::Failed(e) => {
                    return self.failed_output(e, phases_completed, initial_budget)
                }
            }
        } else {
            input.context.clone()
        };

        // === DECIDE ===
        let decision = if start_phase <= CognitivePhase::Decide {
            self.ux.send_progress(40.0, "Planning...");

            match self
                .run_phase(CognitivePhase::Decide, handler.decide(&thought))
                .await
            {
                PhaseResult::Ok(v) => {
                    phases_completed.push(CognitivePhase::Decide);

                    // Gate check: assess risk
                    if let Ok(risk) = handler.assess_risk(&v).await {
                        if risk.needs_approval() {
                            self.ux.send_alert(
                                hydra_core::types::AlertLevel::Warning,
                                "This action needs your approval",
                                None,
                            );
                            // For now, proceed (real implementation would wait for UX decision)
                        }
                    }

                    v
                }
                PhaseResult::TimedOut => {
                    return self.timeout_output(phases_completed, initial_budget)
                }
                PhaseResult::Interrupted => {
                    return self.interrupt_output(phases_completed, initial_budget)
                }
                PhaseResult::BudgetExceeded => {
                    return self.budget_output(phases_completed, initial_budget)
                }
                PhaseResult::Corrupted => {
                    return self.corruption_output(phases_completed, initial_budget)
                }
                PhaseResult::Failed(e) => {
                    return self.failed_output(e, phases_completed, initial_budget)
                }
            }
        } else {
            input.context.clone()
        };

        // Checkpoint before Act (atomic phase)
        self.save_checkpoint(CognitivePhase::Decide, &decision);

        // === ACT ===
        let result = if start_phase <= CognitivePhase::Act {
            self.ux.send_progress(60.0, "Working...");

            match self
                .run_phase(CognitivePhase::Act, handler.act(&decision))
                .await
            {
                PhaseResult::Ok(v) => {
                    phases_completed.push(CognitivePhase::Act);
                    v
                }
                PhaseResult::TimedOut => {
                    return self.timeout_output(phases_completed, initial_budget)
                }
                PhaseResult::Interrupted => {
                    // EC-CL-002: Checkpoint on interrupt during Act
                    self.save_checkpoint(CognitivePhase::Act, &decision);
                    return self.interrupt_output(phases_completed, initial_budget);
                }
                PhaseResult::BudgetExceeded => {
                    return self.budget_output(phases_completed, initial_budget)
                }
                PhaseResult::Corrupted => {
                    return self.corruption_output(phases_completed, initial_budget)
                }
                PhaseResult::Failed(e) => {
                    return self.failed_output(e, phases_completed, initial_budget)
                }
            }
        } else {
            input.context.clone()
        };

        // === LEARN ===
        self.ux.send_progress(90.0, "Learning...");

        let learn_result = match self
            .run_phase(CognitivePhase::Learn, handler.learn(&result))
            .await
        {
            PhaseResult::Ok(v) => {
                phases_completed.push(CognitivePhase::Learn);
                v
            }
            PhaseResult::Failed(_) | PhaseResult::TimedOut => {
                // Learn uses LogAndContinue — don't fail the whole cycle
                phases_completed.push(CognitivePhase::Learn);
                serde_json::json!({"learning": "deferred"})
            }
            PhaseResult::Interrupted => {
                return self.interrupt_output(phases_completed, initial_budget)
            }
            PhaseResult::BudgetExceeded => {
                // Still complete, just didn't learn
                phases_completed.push(CognitivePhase::Learn);
                serde_json::json!({"learning": "skipped_budget"})
            }
            PhaseResult::Corrupted => {
                return self.corruption_output(phases_completed, initial_budget)
            }
        };

        // === COMPLETION ===
        let tokens_used = initial_budget - self.budget.lock().remaining();
        let _ = learn_result; // consumed by learning

        self.ux.send_completion(CompletionSummary {
            headline: "Task completed".into(),
            actions: vec!["Completed cognitive cycle".into()],
            changes: vec![],
            next_steps: vec![],
        });

        // Clear checkpoint on success
        *self.checkpoint.lock() = None;

        let status = if handler.sisters_available() {
            CycleStatus::Completed
        } else {
            CycleStatus::Degraded
        };

        CycleOutput {
            result,
            status,
            tokens_used,
            phases_completed,
        }
    }
}
