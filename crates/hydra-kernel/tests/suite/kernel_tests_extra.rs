use std::time::Duration;

use async_trait::async_trait;
use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment, RiskLevel, TokenBudget};
use hydra_kernel::cognitive_loop::{
    CognitiveLoop, CycleInput, CycleStatus, PhaseHandler,
};
use hydra_kernel::config::KernelConfig;

// ═══════════════════════════════════════════════════════════
// MOCK HANDLER
// ═══════════════════════════════════════════════════════════

struct MockHandler {
    slow_phase: Option<CognitivePhase>,
    slow_duration: Duration,
    failing_phase: Option<CognitivePhase>,
    sisters_up: bool,
}

impl MockHandler {
    fn new() -> Self {
        Self {
            slow_phase: None,
            slow_duration: Duration::from_secs(5),
            failing_phase: None,
            sisters_up: true,
        }
    }

    fn with_failing_phase(mut self, phase: CognitivePhase) -> Self {
        self.failing_phase = Some(phase);
        self
    }

    fn with_sisters_down(mut self) -> Self {
        self.sisters_up = false;
        self
    }

    async fn maybe_slow(&self, phase: CognitivePhase) {
        if self.slow_phase == Some(phase) {
            tokio::time::sleep(self.slow_duration).await;
        }
    }

    fn maybe_fail(&self, phase: CognitivePhase) -> Result<(), HydraError> {
        if self.failing_phase == Some(phase) {
            Err(HydraError::Internal(format!("{phase:?} failed")))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl PhaseHandler for MockHandler {
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        self.maybe_slow(CognitivePhase::Perceive).await;
        self.maybe_fail(CognitivePhase::Perceive)?;
        Ok(serde_json::json!({"perceived": input.text}))
    }

    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.maybe_slow(CognitivePhase::Think).await;
        self.maybe_fail(CognitivePhase::Think)?;
        Ok(serde_json::json!({"thought": perceived}))
    }

    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.maybe_slow(CognitivePhase::Decide).await;
        self.maybe_fail(CognitivePhase::Decide)?;
        Ok(serde_json::json!({"decision": thought}))
    }

    async fn assess_risk(
        &self,
        _decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        Ok(RiskAssessment {
            level: RiskLevel::Low,
            factors: vec![],
            mitigations: vec![],
            requires_approval: false,
        })
    }

    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.maybe_slow(CognitivePhase::Act).await;
        self.maybe_fail(CognitivePhase::Act)?;
        Ok(serde_json::json!({"result": decision}))
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.maybe_slow(CognitivePhase::Learn).await;
        self.maybe_fail(CognitivePhase::Learn)?;
        Ok(serde_json::json!({"learned": result}))
    }

    fn sisters_available(&self) -> bool {
        self.sisters_up
    }
}

// ═══════════════════════════════════════════════════════════
// ADDITIONAL TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_learn_failure_doesnt_fail_cycle() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new().with_failing_phase(CognitivePhase::Learn);
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    // Learn failure uses LogAndContinue — cycle should still complete
    assert!(output.is_ok());
    assert!(output.phases_completed.contains(&CognitivePhase::Learn));
}

#[tokio::test]
async fn test_think_failure_fails_cycle() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new().with_failing_phase(CognitivePhase::Think);
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    assert!(matches!(output.status, CycleStatus::Failed(_)));
}

#[tokio::test]
async fn test_checkpoint_serialization() {
    let checkpoint = hydra_kernel::state::Checkpoint::capture(
        CognitivePhase::Act,
        hydra_kernel::CheckpointLevel::Full,
        serde_json::json!({"state": "mid-execution"}),
        vec!["goal1".into(), "goal2".into()],
        TokenBudget::new(10_000),
    );
    let json = serde_json::to_string(&checkpoint).unwrap();
    let deserialized: hydra_kernel::state::Checkpoint = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.phase, CognitivePhase::Act);
    assert_eq!(deserialized.goals.len(), 2);
}

#[tokio::test]
async fn test_config_phase_timeouts() {
    let config = KernelConfig::default();
    assert_eq!(
        config.phase_timeout(CognitivePhase::Perceive),
        Duration::from_secs(10)
    );
    assert_eq!(
        config.phase_timeout(CognitivePhase::Think),
        Duration::from_secs(60)
    );
    assert_eq!(
        config.phase_timeout(CognitivePhase::Act),
        Duration::from_secs(300)
    );
}

// ═══════════════════════════════════════════════════════════
// BUDGET MANAGER TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_budget_manager_spend_returns_result() {
    use hydra_kernel::BudgetManager;
    let mut mgr = BudgetManager::new(1000);
    // spend() returns Result
    assert!(mgr.spend(500, CognitivePhase::Think).is_ok());
    assert_eq!(mgr.remaining(), 500);

    // Can't afford → returns Err(TokenBudgetExceeded)
    let err = mgr.spend(600, CognitivePhase::Act);
    assert!(err.is_err());
    match err.unwrap_err() {
        hydra_core::error::HydraError::TokenBudgetExceeded { needed, available } => {
            assert_eq!(needed, 600);
            assert_eq!(available, 500);
        }
        other => panic!("Expected TokenBudgetExceeded, got {:?}", other),
    }
}

#[tokio::test]
async fn test_budget_conservation_mode_limits() {
    use hydra_kernel::BudgetManager;
    let mut mgr = BudgetManager::new(1000);
    assert!(!mgr.is_conservation_mode());
    assert!(!mgr.should_skip_reflection());
    assert!(!mgr.should_cache_only());
    assert!(!mgr.should_batch_calls());

    // Spend to trigger conservation mode (< 25%)
    mgr.try_spend(800, CognitivePhase::Think);
    assert!(mgr.is_conservation_mode()); // 200/1000 = 20%

    // Explicitly enter conservation mode to activate limits
    mgr.enter_conservation_mode();
    assert!(mgr.should_skip_reflection());
    assert!(mgr.should_cache_only());
    assert!(mgr.should_batch_calls());
}

#[tokio::test]
async fn test_budget_manager_try_spend() {
    use hydra_kernel::BudgetManager;
    let mut mgr = BudgetManager::new(1000);
    assert!(mgr.try_spend(500, CognitivePhase::Think));
    assert_eq!(mgr.remaining(), 500);
    assert!(!mgr.try_spend(600, CognitivePhase::Act));
    assert_eq!(mgr.remaining(), 500); // unchanged after failed spend
}

// ═══════════════════════════════════════════════════════════
// COGNITIVE STATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cognitive_state_new() {
    let state = hydra_kernel::CognitiveState::new(TokenBudget::new(10_000));
    assert_eq!(state.phase, CognitivePhase::Perceive);
    assert_eq!(state.budget.total, 10_000);
    assert!(state.goals.is_empty());
    assert!(state.beliefs.is_empty());
}

#[test]
fn test_cognitive_state_with_budget() {
    let state = hydra_kernel::CognitiveState::with_budget(TokenBudget::new(50_000));
    assert_eq!(state.budget.total, 50_000);
}

#[test]
fn test_cognitive_state_transition_valid() {
    let mut state = hydra_kernel::CognitiveState::new(TokenBudget::new(1000));
    assert!(state.transition_to(CognitivePhase::Think).is_ok());
    assert_eq!(state.phase, CognitivePhase::Think);
    assert!(state.transition_to(CognitivePhase::Decide).is_ok());
    assert!(state.transition_to(CognitivePhase::Act).is_ok());
    assert!(state.transition_to(CognitivePhase::Learn).is_ok());
    assert!(state.transition_to(CognitivePhase::Perceive).is_ok());
}

#[test]
fn test_cognitive_state_transition_invalid() {
    let mut state = hydra_kernel::CognitiveState::new(TokenBudget::new(1000));
    // Can't jump from Perceive to Act
    assert!(state.transition_to(CognitivePhase::Act).is_err());
    // Can't jump from Perceive to Learn
    assert!(state.transition_to(CognitivePhase::Learn).is_err());
}

#[test]
fn test_cognitive_state_checkpoint_and_restore() {
    let mut state = hydra_kernel::CognitiveState::new(TokenBudget::new(10_000));
    state.transition_to(CognitivePhase::Think).unwrap();
    state.context = serde_json::json!({"key": "value"});

    let checkpoint = state.checkpoint(hydra_kernel::CheckpointLevel::Full);
    assert_eq!(checkpoint.phase, CognitivePhase::Think);

    let restored = hydra_kernel::CognitiveState::restore_from(&checkpoint);
    assert_eq!(restored.phase, CognitivePhase::Think);
    assert_eq!(restored.context, serde_json::json!({"key": "value"}));
    assert_eq!(restored.budget.total, 10_000);
}

#[test]
fn test_cognitive_state_think_to_act_skip() {
    // Think can skip Decide and go straight to Act (compiled skill)
    let mut state = hydra_kernel::CognitiveState::new(TokenBudget::new(1000));
    state.transition_to(CognitivePhase::Think).unwrap();
    assert!(state.transition_to(CognitivePhase::Act).is_ok());
}

#[test]
fn test_cognitive_state_act_to_think_retry() {
    // Act can go back to Think (replanning after failure)
    let mut state = hydra_kernel::CognitiveState::new(TokenBudget::new(1000));
    state.transition_to(CognitivePhase::Think).unwrap();
    state.transition_to(CognitivePhase::Act).unwrap();
    assert!(state.transition_to(CognitivePhase::Think).is_ok());
}

// ═══════════════════════════════════════════════════════════
// UX INTEGRATION — NO PHASE > 5s WITHOUT PROGRESS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_no_phase_over_5s_without_progress() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();
    let mut rx = kernel.ux().subscribe();
    let _output = kernel.run(CycleInput::simple("test"), &handler).await;

    // Collect all updates
    let mut progress_count = 0;
    while let Ok(update) = rx.try_recv() {
        if let hydra_core::types::ProactiveUpdate::Progress { .. } = update {
            progress_count += 1;
        }
    }
    // Should have at least 4 progress updates (Think: 20%, Decide: 40%, Act: 60%, Learn: 90%)
    assert!(
        progress_count >= 4,
        "Expected at least 4 progress updates, got {progress_count}"
    );
}

#[tokio::test]
async fn test_perceive_failure_with_no_sisters_degrades() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new()
        .with_failing_phase(CognitivePhase::Perceive)
        .with_sisters_down();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    // Perceive fails + no sisters → should degrade gracefully
    assert!(output.graceful_degradation());
}
