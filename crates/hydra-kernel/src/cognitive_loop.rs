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
