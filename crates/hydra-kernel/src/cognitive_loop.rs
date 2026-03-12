use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::Notify;

use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment, TokenBudget};
use hydra_ux::proactive::{ProactiveConfig, ProactiveEngine};

use crate::budget::BudgetManager;
use crate::config::KernelConfig;
use crate::state::{Checkpoint, KernelRunState};

/// Input to a cognitive cycle
#[derive(Debug, Clone)]
pub struct CycleInput {
    pub text: String,
    pub context: serde_json::Value,
}

impl CycleInput {
    pub fn simple(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            context: serde_json::Value::Null,
        }
    }
}

/// Output from a cognitive cycle
#[derive(Debug, Clone)]
pub struct CycleOutput {
    pub result: serde_json::Value,
    pub status: CycleStatus,
    pub tokens_used: u64,
    pub phases_completed: Vec<CognitivePhase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleStatus {
    Completed,
    Partial,
    TimedOut,
    Interrupted,
    BudgetExceeded,
    Failed(String),
    Cancelled,
    RecursionDetected,
    Degraded,
}

impl CycleOutput {
    pub fn is_ok(&self) -> bool {
        matches!(self.status, CycleStatus::Completed | CycleStatus::Degraded)
    }

    pub fn is_partial(&self) -> bool {
        matches!(self.status, CycleStatus::Partial)
    }

    pub fn budget_exceeded(&self) -> bool {
        matches!(self.status, CycleStatus::BudgetExceeded)
    }

    pub fn timed_out(&self) -> bool {
        matches!(self.status, CycleStatus::TimedOut)
    }

    pub fn used_fallback(&self) -> bool {
        matches!(self.status, CycleStatus::Degraded)
    }

    pub fn continued_from_checkpoint(&self) -> bool {
        // Will be set by resume_from
        self.phases_completed.first() != Some(&CognitivePhase::Perceive)
    }

    pub fn recursion_detected(&self) -> bool {
        matches!(self.status, CycleStatus::RecursionDetected)
    }

    pub fn depth_limited(&self) -> bool {
        self.recursion_detected()
    }

    pub fn graceful_degradation(&self) -> bool {
        matches!(self.status, CycleStatus::Degraded)
    }

    pub fn detected_corruption(&self) -> bool {
        matches!(self.status, CycleStatus::Failed(ref s) if s.contains("corruption"))
    }

    pub fn recovered(&self) -> bool {
        matches!(self.status, CycleStatus::Degraded)
    }
}

/// Trait for phase handlers — implement this to customize what each phase does
#[async_trait]
pub trait PhaseHandler: Send + Sync {
    /// Perceive: gather input, understand context
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError>;
    /// Think: reason about the perceived input
    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError>;
    /// Decide: choose an action plan
    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError>;
    /// Assess risk of the decided action
    async fn assess_risk(&self, decision: &serde_json::Value)
        -> Result<RiskAssessment, HydraError>;
    /// Act: execute the decision
    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError>;
    /// Learn: update beliefs/memory from the result
    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError>;
    /// Check if sisters are available
    fn sisters_available(&self) -> bool {
        true
    }
}

/// The cognitive loop — the "mind runtime"
pub struct CognitiveLoop {
    pub(crate) config: KernelConfig,
    pub(crate) ux: Arc<ProactiveEngine>,
    pub(crate) budget: Arc<Mutex<BudgetManager>>,
    pub(crate) run_state: Arc<Mutex<KernelRunState>>,
    pub(crate) checkpoint: Arc<Mutex<Option<Checkpoint>>>,
    pub(crate) interrupted: Arc<AtomicBool>,
    pub(crate) recursion_depth: Arc<AtomicUsize>,
    pub(crate) corruption_flag: Arc<AtomicBool>,
    pub(crate) interrupt_notify: Arc<Notify>,
}

impl CognitiveLoop {
    pub fn new(config: KernelConfig) -> Self {
        let budget = BudgetManager::new(config.token_budget);
        Self {
            ux: Arc::new(ProactiveEngine::new(ProactiveConfig::default())),
            budget: Arc::new(Mutex::new(budget)),
            run_state: Arc::new(Mutex::new(KernelRunState::Idle)),
            checkpoint: Arc::new(Mutex::new(None)),
            interrupted: Arc::new(AtomicBool::new(false)),
            recursion_depth: Arc::new(AtomicUsize::new(0)),
            corruption_flag: Arc::new(AtomicBool::new(false)),
            interrupt_notify: Arc::new(Notify::new()),
            config,
        }
    }

    pub fn with_budget(self, budget: TokenBudget) -> Self {
        *self.budget.lock() = BudgetManager::from_budget(budget);
        self
    }

    pub fn ux(&self) -> &ProactiveEngine {
        &self.ux
    }

    pub fn run_state(&self) -> KernelRunState {
        *self.run_state.lock()
    }

    pub fn has_checkpoint(&self) -> bool {
        self.checkpoint.lock().is_some()
    }

    pub fn is_corrupted(&self) -> bool {
        self.corruption_flag.load(Ordering::SeqCst)
    }

    pub fn interrupt(&self) {
        self.interrupted.store(true, Ordering::SeqCst);
        self.interrupt_notify.notify_one();
    }

    /// Simulate state corruption (for testing EC-CL-006)
    pub fn corrupt_state(&self) {
        self.corruption_flag.store(true, Ordering::SeqCst);
    }

    /// Run the full cognitive cycle
    pub async fn run(&self, input: CycleInput, handler: &dyn PhaseHandler) -> CycleOutput {
        // Check recursion depth (EC-CL-010)
        let depth = self.recursion_depth.fetch_add(1, Ordering::SeqCst);
        if depth >= self.config.max_recursion_depth {
            self.recursion_depth.fetch_sub(1, Ordering::SeqCst);
            return CycleOutput {
                result: serde_json::json!({"error": "recursion depth exceeded"}),
                status: CycleStatus::RecursionDetected,
                tokens_used: 0,
                phases_completed: vec![],
            };
        }

        self.interrupted.store(false, Ordering::SeqCst);
        *self.run_state.lock() = KernelRunState::Running;

        let result = self
            .run_phases(input, handler, CognitivePhase::Perceive)
            .await;

        self.recursion_depth.fetch_sub(1, Ordering::SeqCst);
        *self.run_state.lock() = match result.status {
            CycleStatus::Completed | CycleStatus::Degraded => KernelRunState::Completed,
            CycleStatus::Interrupted => KernelRunState::Interrupted,
            CycleStatus::Failed(_) => KernelRunState::Failed,
            _ => KernelRunState::Completed,
        };

        result
    }

    /// Resume from a checkpoint (EC-CL-009)
    pub async fn resume_from(
        &self,
        checkpoint: Checkpoint,
        handler: &dyn PhaseHandler,
    ) -> CycleOutput {
        // Restore budget from checkpoint
        *self.budget.lock() = BudgetManager::from_budget(checkpoint.budget.clone());

        // Determine which phase to resume from (next phase after checkpoint)
        let resume_phase = next_phase(checkpoint.phase);

        let input = CycleInput {
            text: String::new(),
            context: checkpoint.context.clone(),
        };

        *self.checkpoint.lock() = Some(checkpoint);
        *self.run_state.lock() = KernelRunState::Running;

        self.run_phases(input, handler, resume_phase).await
    }

    // run_phases — extracted to cognitive_loop_phases.rs
    // run_phase, estimated_phase_cost, save_checkpoint, output builders
    // — all extracted to cognitive_loop_output.rs
}

pub(crate) enum PhaseResult {
    Ok(serde_json::Value),
    TimedOut,
    Interrupted,
    BudgetExceeded,
    Corrupted,
    Failed(String),
}

fn next_phase(phase: CognitivePhase) -> CognitivePhase {
    match phase {
        CognitivePhase::Perceive => CognitivePhase::Think,
        CognitivePhase::Think => CognitivePhase::Decide,
        CognitivePhase::Decide => CognitivePhase::Act,
        CognitivePhase::Act => CognitivePhase::Learn,
        CognitivePhase::Learn => CognitivePhase::Perceive,
    }
}

#[cfg(test)]
#[path = "cognitive_loop_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cognitive_loop_tests_extra.rs"]
mod tests_extra;
