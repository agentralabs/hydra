//! Cognitive loop helpers — extracted from cognitive_loop.rs for file size.
//! Contains output builders, run_phase, estimated_phase_cost, save_checkpoint.

use std::sync::atomic::Ordering;

use hydra_core::error::HydraError;
use hydra_core::types::CognitivePhase;

use crate::config::CheckpointLevel;
use crate::state::Checkpoint;
use super::cognitive_loop::{CognitiveLoop, CycleOutput, CycleStatus, PhaseResult};

impl CognitiveLoop {
    pub(crate) fn timeout_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
        self.ux.send_alert(
            hydra_core::types::AlertLevel::Warning,
            "The operation timed out",
            Some("Try again or simplify the request".into()),
        );
        CycleOutput {
            result: serde_json::json!({"timeout": true}),
            status: CycleStatus::TimedOut,
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    pub(crate) fn interrupt_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
        CycleOutput {
            result: serde_json::json!({"interrupted": true}),
            status: CycleStatus::Interrupted,
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    pub(crate) fn budget_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
        self.ux.send_alert(
            hydra_core::types::AlertLevel::Warning,
            "Token budget exceeded",
            Some("Switching to conservation mode".into()),
        );
        CycleOutput {
            result: serde_json::json!({"budget_exceeded": true}),
            status: CycleStatus::BudgetExceeded,
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    pub(crate) fn corruption_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
        self.ux.send_alert(
            hydra_core::types::AlertLevel::Error,
            "State corruption detected",
            Some("Attempting recovery".into()),
        );
        CycleOutput {
            result: serde_json::json!({"corruption": true}),
            status: CycleStatus::Failed("state corruption detected".into()),
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    pub(crate) fn failed_output(
        &self,
        error: String,
        phases: Vec<CognitivePhase>,
        initial_budget: u64,
    ) -> CycleOutput {
        self.ux
            .send_alert(hydra_core::types::AlertLevel::Error, &error, None);
        CycleOutput {
            result: serde_json::json!({"error": error}),
            status: CycleStatus::Failed(error),
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    /// Run a single phase with timeout, budget check, interruption, and corruption detection
    pub(crate) async fn run_phase<F>(&self, phase: CognitivePhase, future: F) -> PhaseResult
    where
        F: std::future::Future<Output = Result<serde_json::Value, HydraError>>,
    {
        // Check corruption (EC-CL-006)
        if self.corruption_flag.load(Ordering::SeqCst) {
            return PhaseResult::Corrupted;
        }

        // Check budget (EC-CL-007)
        let phase_cost = self.estimated_phase_cost(phase);
        if !self.budget.lock().can_afford(phase_cost) {
            return PhaseResult::BudgetExceeded;
        }

        // Check interruption (EC-CL-002)
        if self.interrupted.load(Ordering::SeqCst) {
            return PhaseResult::Interrupted;
        }

        let timeout = self.config.phase_timeout(phase);

        // Run with timeout (EC-CL-001)
        match tokio::time::timeout(timeout, future).await {
            Ok(Ok(value)) => {
                self.budget.lock().try_spend(phase_cost, phase);
                PhaseResult::Ok(value)
            }
            Ok(Err(e)) => PhaseResult::Failed(e.to_string()),
            Err(_) => PhaseResult::TimedOut,
        }
    }

    pub(crate) fn estimated_phase_cost(&self, phase: CognitivePhase) -> u64 {
        match phase {
            CognitivePhase::Perceive => 100,
            CognitivePhase::Think => 500,
            CognitivePhase::Decide => 200,
            CognitivePhase::Act => 100,
            CognitivePhase::Learn => 100,
        }
    }

    pub(crate) fn save_checkpoint(&self, phase: CognitivePhase, context: &serde_json::Value) {
        let budget = self.budget.lock().budget().clone();
        let level = self
            .config
            .phase_configs
            .get(&phase)
            .map(|c| c.checkpoint_level)
            .unwrap_or(CheckpointLevel::Full);
        *self.checkpoint.lock() = Some(Checkpoint::capture(
            phase,
            level,
            context.clone(),
            vec![],
            budget,
        ));
    }
}
