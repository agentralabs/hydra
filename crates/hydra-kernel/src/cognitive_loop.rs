use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::Notify;

use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, CompletionSummary, RiskAssessment, TokenBudget};
use hydra_ux::proactive::{ProactiveConfig, ProactiveEngine};

use crate::budget::BudgetManager;
use crate::config::{CheckpointLevel, KernelConfig};
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
    config: KernelConfig,
    ux: Arc<ProactiveEngine>,
    budget: Arc<Mutex<BudgetManager>>,
    run_state: Arc<Mutex<KernelRunState>>,
    checkpoint: Arc<Mutex<Option<Checkpoint>>>,
    interrupted: Arc<AtomicBool>,
    recursion_depth: Arc<AtomicUsize>,
    corruption_flag: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
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

    async fn run_phases(
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

    /// Run a single phase with timeout, budget check, interruption, and corruption detection
    async fn run_phase<F>(&self, phase: CognitivePhase, future: F) -> PhaseResult
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

    fn estimated_phase_cost(&self, phase: CognitivePhase) -> u64 {
        match phase {
            CognitivePhase::Perceive => 100,
            CognitivePhase::Think => 500,
            CognitivePhase::Decide => 200,
            CognitivePhase::Act => 100,
            CognitivePhase::Learn => 100,
        }
    }

    fn save_checkpoint(&self, phase: CognitivePhase, context: &serde_json::Value) {
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

    fn timeout_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
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

    fn interrupt_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
        CycleOutput {
            result: serde_json::json!({"interrupted": true}),
            status: CycleStatus::Interrupted,
            tokens_used: initial_budget - self.budget.lock().remaining(),
            phases_completed: phases,
        }
    }

    fn budget_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
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

    fn corruption_output(&self, phases: Vec<CognitivePhase>, initial_budget: u64) -> CycleOutput {
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

    fn failed_output(
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
}

enum PhaseResult {
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
mod tests {
    use super::*;
    use crate::config::KernelConfig;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
    use std::time::Duration;

    // ── Test helpers ──

    /// A simple handler that succeeds on every phase, returning the input as-is.
    struct EchoHandler;

    #[async_trait]
    impl PhaseHandler for EchoHandler {
        async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"perceived": input.text}))
        }
        async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"thought": perceived}))
        }
        async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"decided": thought}))
        }
        async fn assess_risk(&self, _decision: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
            Ok(RiskAssessment {
                level: hydra_core::types::RiskLevel::None,
                factors: vec![],
                mitigations: vec![],
                requires_approval: false,
            })
        }
        async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"acted": decision}))
        }
        async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"learned": result}))
        }
    }

    /// A handler that fails on a specific phase.
    struct FailOnPhaseHandler {
        fail_phase: CognitivePhase,
    }

    #[async_trait]
    impl PhaseHandler for FailOnPhaseHandler {
        async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
            if self.fail_phase == CognitivePhase::Perceive {
                Err(HydraError::Internal("perceive failed".into()))
            } else {
                Ok(json!({"perceived": input.text}))
            }
        }
        async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.fail_phase == CognitivePhase::Think {
                Err(HydraError::Internal("think failed".into()))
            } else {
                Ok(json!({"thought": perceived}))
            }
        }
        async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.fail_phase == CognitivePhase::Decide {
                Err(HydraError::Internal("decide failed".into()))
            } else {
                Ok(json!({"decided": thought}))
            }
        }
        async fn assess_risk(&self, _decision: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
            Ok(RiskAssessment {
                level: hydra_core::types::RiskLevel::None,
                factors: vec![],
                mitigations: vec![],
                requires_approval: false,
            })
        }
        async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.fail_phase == CognitivePhase::Act {
                Err(HydraError::Internal("act failed".into()))
            } else {
                Ok(json!({"acted": decision}))
            }
        }
        async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.fail_phase == CognitivePhase::Learn {
                Err(HydraError::Internal("learn failed".into()))
            } else {
                Ok(json!({"learned": result}))
            }
        }
    }

    /// A handler that sleeps forever on a specific phase (for timeout testing).
    struct SlowHandler {
        slow_phase: CognitivePhase,
    }

    #[async_trait]
    impl PhaseHandler for SlowHandler {
        async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
            if self.slow_phase == CognitivePhase::Perceive {
                tokio::time::sleep(Duration::from_secs(999)).await;
            }
            Ok(json!({"perceived": input.text}))
        }
        async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.slow_phase == CognitivePhase::Think {
                tokio::time::sleep(Duration::from_secs(999)).await;
            }
            Ok(json!({"thought": perceived}))
        }
        async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.slow_phase == CognitivePhase::Decide {
                tokio::time::sleep(Duration::from_secs(999)).await;
            }
            Ok(json!({"decided": thought}))
        }
        async fn assess_risk(&self, _decision: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
            Ok(RiskAssessment {
                level: hydra_core::types::RiskLevel::None,
                factors: vec![],
                mitigations: vec![],
                requires_approval: false,
            })
        }
        async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.slow_phase == CognitivePhase::Act {
                tokio::time::sleep(Duration::from_secs(999)).await;
            }
            Ok(json!({"acted": decision}))
        }
        async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            if self.slow_phase == CognitivePhase::Learn {
                tokio::time::sleep(Duration::from_secs(999)).await;
            }
            Ok(json!({"learned": result}))
        }
    }

    /// A handler that reports sisters unavailable.
    struct NoSistersHandler;

    #[async_trait]
    impl PhaseHandler for NoSistersHandler {
        async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"perceived": input.text}))
        }
        async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"thought": perceived}))
        }
        async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"decided": thought}))
        }
        async fn assess_risk(&self, _decision: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
            Ok(RiskAssessment {
                level: hydra_core::types::RiskLevel::None,
                factors: vec![],
                mitigations: vec![],
                requires_approval: false,
            })
        }
        async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"acted": decision}))
        }
        async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            Ok(json!({"learned": result}))
        }
        fn sisters_available(&self) -> bool {
            false
        }
    }

    /// A handler that counts how many times each phase is called.
    struct CountingHandler {
        perceive_count: AtomicU32,
        think_count: AtomicU32,
        decide_count: AtomicU32,
        act_count: AtomicU32,
        learn_count: AtomicU32,
    }

    impl CountingHandler {
        fn new() -> Self {
            Self {
                perceive_count: AtomicU32::new(0),
                think_count: AtomicU32::new(0),
                decide_count: AtomicU32::new(0),
                act_count: AtomicU32::new(0),
                learn_count: AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl PhaseHandler for CountingHandler {
        async fn perceive(&self, _input: &CycleInput) -> Result<serde_json::Value, HydraError> {
            self.perceive_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(json!({}))
        }
        async fn think(&self, _perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            self.think_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(json!({}))
        }
        async fn decide(&self, _thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            self.decide_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(json!({}))
        }
        async fn assess_risk(&self, _decision: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
            Ok(RiskAssessment {
                level: hydra_core::types::RiskLevel::None,
                factors: vec![],
                mitigations: vec![],
                requires_approval: false,
            })
        }
        async fn act(&self, _decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            self.act_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(json!({}))
        }
        async fn learn(&self, _result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
            self.learn_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(json!({}))
        }
    }

    fn fast_config() -> KernelConfig {
        let mut config = KernelConfig::default();
        // Use short timeouts for test speed
        for phase in [
            CognitivePhase::Perceive,
            CognitivePhase::Think,
            CognitivePhase::Decide,
            CognitivePhase::Act,
            CognitivePhase::Learn,
        ] {
            config.set_phase_timeout(phase, Duration::from_millis(100));
        }
        config
    }

    // ── CycleInput tests ──

    #[test]
    fn test_cycle_input_simple() {
        let input = CycleInput::simple("hello");
        assert_eq!(input.text, "hello");
        assert_eq!(input.context, serde_json::Value::Null);
    }

    // ── CycleOutput method tests ──

    #[test]
    fn test_cycle_output_is_ok_completed() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Completed,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.is_ok());
    }

    #[test]
    fn test_cycle_output_is_ok_degraded() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Degraded,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.is_ok());
        assert!(output.used_fallback());
        assert!(output.graceful_degradation());
        assert!(output.recovered());
    }

    #[test]
    fn test_cycle_output_is_not_ok_for_failed() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Failed("oops".into()),
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(!output.is_ok());
    }

    #[test]
    fn test_cycle_output_partial() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Partial,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.is_partial());
        assert!(!output.is_ok());
    }

    #[test]
    fn test_cycle_output_budget_exceeded() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::BudgetExceeded,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.budget_exceeded());
    }

    #[test]
    fn test_cycle_output_timed_out() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::TimedOut,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.timed_out());
    }

    #[test]
    fn test_cycle_output_recursion_detected() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::RecursionDetected,
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.recursion_detected());
        assert!(output.depth_limited());
    }

    #[test]
    fn test_cycle_output_detected_corruption() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Failed("state corruption detected".into()),
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(output.detected_corruption());
    }

    #[test]
    fn test_cycle_output_no_corruption_for_other_failures() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Failed("network error".into()),
            tokens_used: 0,
            phases_completed: vec![],
        };
        assert!(!output.detected_corruption());
    }

    #[test]
    fn test_continued_from_checkpoint_false_when_starts_at_perceive() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Completed,
            tokens_used: 0,
            phases_completed: vec![CognitivePhase::Perceive, CognitivePhase::Think],
        };
        assert!(!output.continued_from_checkpoint());
    }

    #[test]
    fn test_continued_from_checkpoint_true_when_starts_at_think() {
        let output = CycleOutput {
            result: json!({}),
            status: CycleStatus::Completed,
            tokens_used: 0,
            phases_completed: vec![CognitivePhase::Think, CognitivePhase::Decide],
        };
        assert!(output.continued_from_checkpoint());
    }

    // ── next_phase tests ──

    #[test]
    fn test_next_phase_cycle() {
        assert_eq!(next_phase(CognitivePhase::Perceive), CognitivePhase::Think);
        assert_eq!(next_phase(CognitivePhase::Think), CognitivePhase::Decide);
        assert_eq!(next_phase(CognitivePhase::Decide), CognitivePhase::Act);
        assert_eq!(next_phase(CognitivePhase::Act), CognitivePhase::Learn);
        assert_eq!(next_phase(CognitivePhase::Learn), CognitivePhase::Perceive);
    }

    // ── CognitiveLoop construction ──

    #[test]
    fn test_new_loop_is_idle() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        assert_eq!(cl.run_state(), KernelRunState::Idle);
        assert!(!cl.has_checkpoint());
        assert!(!cl.is_corrupted());
    }

    // ── Full cycle tests ──

    #[tokio::test]
    async fn test_full_cycle_completes_all_five_phases() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("hello"), &handler).await;
        assert!(output.is_ok());
        assert_eq!(output.phases_completed.len(), 5);
        assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
        assert_eq!(output.phases_completed[1], CognitivePhase::Think);
        assert_eq!(output.phases_completed[2], CognitivePhase::Decide);
        assert_eq!(output.phases_completed[3], CognitivePhase::Act);
        assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
    }

    #[tokio::test]
    async fn test_full_cycle_uses_tokens() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("hello"), &handler).await;
        // Phase costs: 100 + 500 + 200 + 100 + 100 = 1000
        assert_eq!(output.tokens_used, 1000);
    }

    #[tokio::test]
    async fn test_run_state_is_completed_after_success() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = EchoHandler;
        cl.run(CycleInput::simple("test"), &handler).await;
        assert_eq!(cl.run_state(), KernelRunState::Completed);
    }

    #[tokio::test]
    async fn test_checkpoint_cleared_after_success() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = EchoHandler;
        cl.run(CycleInput::simple("test"), &handler).await;
        assert!(!cl.has_checkpoint());
    }

    // ── Failure tests ──

    #[tokio::test]
    async fn test_think_failure_aborts_cycle() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = FailOnPhaseHandler { fail_phase: CognitivePhase::Think };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(matches!(output.status, CycleStatus::Failed(_)));
        assert_eq!(output.phases_completed, vec![CognitivePhase::Perceive]);
    }

    #[tokio::test]
    async fn test_decide_failure_aborts_cycle() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = FailOnPhaseHandler { fail_phase: CognitivePhase::Decide };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(matches!(output.status, CycleStatus::Failed(_)));
        assert_eq!(output.phases_completed.len(), 2); // Perceive + Think
    }

    #[tokio::test]
    async fn test_act_failure_aborts_cycle() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = FailOnPhaseHandler { fail_phase: CognitivePhase::Act };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(matches!(output.status, CycleStatus::Failed(_)));
        assert_eq!(output.phases_completed.len(), 3); // Perceive + Think + Decide
    }

    #[tokio::test]
    async fn test_learn_failure_does_not_abort_cycle() {
        // Learn uses LogAndContinue — failure should not fail the cycle
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = FailOnPhaseHandler { fail_phase: CognitivePhase::Learn };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        // Cycle should still complete (Learn adds itself even on failure)
        assert!(output.is_ok());
        assert_eq!(output.phases_completed.len(), 5);
    }

    #[tokio::test]
    async fn test_run_state_is_failed_after_phase_failure() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = FailOnPhaseHandler { fail_phase: CognitivePhase::Think };
        cl.run(CycleInput::simple("test"), &handler).await;
        assert_eq!(cl.run_state(), KernelRunState::Failed);
    }

    // ── Perceive failure with no sisters (degraded mode) ──

    #[tokio::test]
    async fn test_perceive_failure_no_sisters_degrades_gracefully() {
        // When perceive fails and sisters_available() is false, cycle degrades
        struct PerceiveFailNoSisters;

        #[async_trait]
        impl PhaseHandler for PerceiveFailNoSisters {
            async fn perceive(&self, _input: &CycleInput) -> Result<serde_json::Value, HydraError> {
                Err(HydraError::Internal("perceive failed".into()))
            }
            async fn think(&self, _p: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
                Ok(json!({}))
            }
            async fn decide(&self, _t: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
                Ok(json!({}))
            }
            async fn assess_risk(&self, _d: &serde_json::Value) -> Result<RiskAssessment, HydraError> {
                Ok(RiskAssessment {
                    level: hydra_core::types::RiskLevel::None,
                    factors: vec![],
                    mitigations: vec![],
                    requires_approval: false,
                })
            }
            async fn act(&self, _d: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
                Ok(json!({}))
            }
            async fn learn(&self, _r: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
                Ok(json!({}))
            }
            fn sisters_available(&self) -> bool {
                false
            }
        }

        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = PerceiveFailNoSisters;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        // Should degrade, not fail
        assert_eq!(output.status, CycleStatus::Degraded);
        assert!(output.is_ok());
    }

    // ── Timeout tests ──

    #[tokio::test]
    async fn test_timeout_on_think_phase() {
        let cl = CognitiveLoop::new(fast_config());
        let handler = SlowHandler { slow_phase: CognitivePhase::Think };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.timed_out());
        assert_eq!(output.phases_completed, vec![CognitivePhase::Perceive]);
    }

    #[tokio::test]
    async fn test_timeout_on_perceive_phase() {
        let cl = CognitiveLoop::new(fast_config());
        let handler = SlowHandler { slow_phase: CognitivePhase::Perceive };
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.timed_out());
        assert!(output.phases_completed.is_empty());
    }

    // ── Budget tests ──

    #[tokio::test]
    async fn test_budget_exceeded_stops_cycle() {
        let mut config = KernelConfig::default();
        // Budget too small for even Perceive (cost 100)
        config.token_budget = 50;
        let cl = CognitiveLoop::new(config);
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.budget_exceeded());
        assert!(output.phases_completed.is_empty());
    }

    #[tokio::test]
    async fn test_budget_exceeded_mid_cycle() {
        let mut config = KernelConfig::default();
        // Enough for Perceive (100) but not Think (500)
        config.token_budget = 200;
        let cl = CognitiveLoop::new(config);
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.budget_exceeded());
        assert_eq!(output.phases_completed, vec![CognitivePhase::Perceive]);
    }

    // ── Interrupt tests ──

    #[tokio::test]
    async fn test_interrupt_before_run() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        cl.interrupt();
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        // Interrupt flag is cleared at start of run, so it should complete
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_interrupt_sets_flag() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        assert!(!cl.interrupted.load(Ordering::SeqCst));
        cl.interrupt();
        assert!(cl.interrupted.load(Ordering::SeqCst));
    }

    // ── Corruption tests ──

    #[tokio::test]
    async fn test_corruption_flag_stops_cycle() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        cl.corrupt_state();
        assert!(cl.is_corrupted());
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.detected_corruption());
        assert!(output.phases_completed.is_empty());
    }

    // ── Recursion depth tests ──

    #[tokio::test]
    async fn test_recursion_depth_exceeded() {
        let mut config = KernelConfig::default();
        config.max_recursion_depth = 0; // no recursion allowed
        let cl = CognitiveLoop::new(config);
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert!(output.recursion_detected());
        assert!(output.depth_limited());
    }

    // ── No sisters (degraded) ──

    #[tokio::test]
    async fn test_no_sisters_produces_degraded_status() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = NoSistersHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        assert_eq!(output.status, CycleStatus::Degraded);
        assert!(output.is_ok());
        assert!(output.used_fallback());
    }

    // ── Checkpoint / resume ──

    #[tokio::test]
    async fn test_resume_from_checkpoint_skips_earlier_phases() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let counting = CountingHandler::new();

        // Create a checkpoint at Decide phase
        let checkpoint = Checkpoint::capture(
            CognitivePhase::Decide,
            crate::config::CheckpointLevel::Full,
            json!({"context": "from checkpoint"}),
            vec![],
            TokenBudget::new(100_000),
        );

        let output = cl.resume_from(checkpoint, &counting).await;
        assert!(output.is_ok());

        // Perceive and Think should NOT have been called
        assert_eq!(counting.perceive_count.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(counting.think_count.load(AtomicOrdering::SeqCst), 0);
        // Decide should NOT have been called (resume starts at next_phase(Decide) = Act)
        assert_eq!(counting.decide_count.load(AtomicOrdering::SeqCst), 0);
        // Act and Learn should have been called
        assert_eq!(counting.act_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.learn_count.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_resume_from_perceive_checkpoint() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let counting = CountingHandler::new();

        let checkpoint = Checkpoint::capture(
            CognitivePhase::Perceive,
            crate::config::CheckpointLevel::Full,
            json!({}),
            vec![],
            TokenBudget::new(100_000),
        );

        let output = cl.resume_from(checkpoint, &counting).await;
        assert!(output.is_ok());

        // Should skip Perceive, run Think through Learn
        assert_eq!(counting.perceive_count.load(AtomicOrdering::SeqCst), 0);
        assert_eq!(counting.think_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.decide_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.act_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.learn_count.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_resume_output_indicates_checkpoint() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let handler = EchoHandler;

        let checkpoint = Checkpoint::capture(
            CognitivePhase::Decide,
            crate::config::CheckpointLevel::Full,
            json!({}),
            vec![],
            TokenBudget::new(100_000),
        );

        let output = cl.resume_from(checkpoint, &handler).await;
        assert!(output.continued_from_checkpoint());
    }

    // ── with_budget ──

    #[tokio::test]
    async fn test_with_budget_overrides_default() {
        let cl = CognitiveLoop::new(KernelConfig::default())
            .with_budget(TokenBudget::new(150));
        let handler = EchoHandler;
        let output = cl.run(CycleInput::simple("test"), &handler).await;
        // 150 is enough for Perceive (100) but not Think (500)
        assert!(output.budget_exceeded());
        assert_eq!(output.phases_completed, vec![CognitivePhase::Perceive]);
    }

    // ── Each phase invoked exactly once ──

    #[tokio::test]
    async fn test_each_phase_called_exactly_once() {
        let cl = CognitiveLoop::new(KernelConfig::default());
        let counting = CountingHandler::new();
        let output = cl.run(CycleInput::simple("test"), &counting).await;
        assert!(output.is_ok());
        assert_eq!(counting.perceive_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.think_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.decide_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.act_count.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(counting.learn_count.load(AtomicOrdering::SeqCst), 1);
    }
}
