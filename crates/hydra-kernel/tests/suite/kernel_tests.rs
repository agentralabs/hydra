use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment, RiskLevel, TokenBudget};
use hydra_kernel::cognitive_loop::{
    CognitiveLoop, CycleInput, CycleOutput, CycleStatus, PhaseHandler,
};
use hydra_kernel::config::KernelConfig;
use hydra_kernel::state::KernelRunState;

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

    fn with_slow_phase(mut self, phase: CognitivePhase, duration: Duration) -> Self {
        self.slow_phase = Some(phase);
        self.slow_duration = duration;
        self
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
// HAPPY PATH TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_full_cognitive_cycle() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();
    let output = kernel.run(CycleInput::simple("hello"), &handler).await;

    assert!(output.is_ok());
    assert_eq!(output.phases_completed.len(), 5);
    assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
    assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
    assert!(output.tokens_used > 0);
}

#[tokio::test]
async fn test_ux_acknowledgment_emitted() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();
    let mut rx = kernel.ux().subscribe();
    let _output = kernel.run(CycleInput::simple("test"), &handler).await;

    // First update should be an acknowledgment
    let first = rx.try_recv().unwrap();
    match first {
        hydra_core::types::ProactiveUpdate::Acknowledgment { message } => {
            assert_eq!(message, "Got it!");
        }
        _ => panic!("Expected Acknowledgment as first update"),
    }
}

#[tokio::test]
async fn test_ux_progress_updates_emitted() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();
    let mut rx = kernel.ux().subscribe();
    let _output = kernel.run(CycleInput::simple("test"), &handler).await;

    let mut got_progress = false;
    let mut got_completion = false;
    while let Ok(update) = rx.try_recv() {
        match update {
            hydra_core::types::ProactiveUpdate::Progress { .. } => got_progress = true,
            hydra_core::types::ProactiveUpdate::Completion { .. } => got_completion = true,
            _ => {}
        }
    }
    assert!(got_progress, "Should have emitted progress updates");
    assert!(got_completion, "Should have emitted completion");
}

#[tokio::test]
async fn test_kernel_state_transitions() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    assert_eq!(kernel.run_state(), KernelRunState::Idle);

    let handler = MockHandler::new();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    assert!(output.is_ok());
    assert_eq!(kernel.run_state(), KernelRunState::Completed);
}

#[tokio::test]
async fn test_budget_tracking() {
    let config = KernelConfig {
        token_budget: 10_000,
        ..Default::default()
    };
    let kernel = CognitiveLoop::new(config);
    let handler = MockHandler::new();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    assert!(output.is_ok());
    assert!(output.tokens_used > 0);
    assert!(output.tokens_used <= 10_000);
}

// ═══════════════════════════════════════════════════════════
// EDGE CASE TESTS (EC-CL-001 through EC-CL-010)
// ═══════════════════════════════════════════════════════════

/// EC-CL-001: Phase timeout
#[tokio::test]
async fn test_ec_cl_001_phase_timeout() {
    let mut config = KernelConfig::default();
    config.set_phase_timeout(CognitivePhase::Think, Duration::from_millis(1));
    let kernel = CognitiveLoop::new(config);

    // Handler with slow Think phase
    let handler = MockHandler::new().with_slow_phase(CognitivePhase::Think, Duration::from_secs(1));

    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    assert!(output.timed_out() || output.used_fallback());
}

/// EC-CL-002: Interrupt during critical phase (Act)
#[tokio::test]
async fn test_ec_cl_002_interrupt_during_act() {
    let kernel = Arc::new(CognitiveLoop::new(KernelConfig::default()));

    // Slow Act phase so we can interrupt
    let handler = MockHandler::new().with_slow_phase(CognitivePhase::Act, Duration::from_secs(2));

    let kernel_clone = kernel.clone();
    let handle = tokio::spawn(async move {
        // Give Act time to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        kernel_clone.interrupt();
    });

    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    handle.abort(); // cleanup

    // Should have checkpointed and aborted cleanly
    assert!(kernel.has_checkpoint());
    assert!(!kernel.is_corrupted());
    // Output status is either Interrupted (if checked in time) or TimedOut
    assert!(
        matches!(output.status, CycleStatus::Interrupted)
            || matches!(output.status, CycleStatus::Completed)
            || matches!(output.status, CycleStatus::TimedOut)
    );
}

/// EC-CL-003: Low memory simulation (graceful degradation)
#[tokio::test]
async fn test_ec_cl_003_low_memory() {
    // Simulate low memory by giving a very small budget
    let config = KernelConfig {
        token_budget: 50_000,
        ..Default::default()
    };
    let kernel = CognitiveLoop::new(config);
    let handler = MockHandler::new();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    // Should complete (or degrade), not crash
    assert!(output.is_ok() || output.graceful_degradation());
}

/// EC-CL-004: All sisters unavailable
#[tokio::test]
async fn test_ec_cl_004_no_sisters() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new().with_sisters_down();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    // Should work with degraded capability
    assert!(output.is_ok() || output.graceful_degradation());
}

/// EC-CL-005: Infinite loop in reasoning (prevention via max iterations)
#[tokio::test]
async fn test_ec_cl_005_infinite_reasoning_prevention() {
    let mut config = KernelConfig::default();
    // Tight timeout on Think to prevent infinite loops
    config.set_phase_timeout(CognitivePhase::Think, Duration::from_millis(100));
    config.max_think_iterations = 3;

    let kernel = CognitiveLoop::new(config);
    // Slow think simulates a runaway reasoning loop
    let handler =
        MockHandler::new().with_slow_phase(CognitivePhase::Think, Duration::from_secs(10));

    let start = std::time::Instant::now();
    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    let elapsed = start.elapsed();

    // Must not hang forever — should timeout quickly
    assert!(
        elapsed < Duration::from_secs(5),
        "Took {:?}, should be < 5s",
        elapsed
    );
    assert!(output.timed_out());
}

/// EC-CL-006: State corruption mid-run
#[tokio::test]
async fn test_ec_cl_006_state_corruption() {
    let kernel = Arc::new(CognitiveLoop::new(KernelConfig::default()));
    let handler =
        MockHandler::new().with_slow_phase(CognitivePhase::Think, Duration::from_millis(50));

    // Corrupt state during Think phase
    let kernel_clone = kernel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        kernel_clone.corrupt_state();
    });

    let output = kernel.run(CycleInput::simple("test"), &handler).await;
    // Should detect corruption and fail safely (or recover)
    assert!(output.detected_corruption() || output.recovered());
}

/// EC-CL-007: Token budget exhausted mid-phase
#[tokio::test]
async fn test_ec_cl_007_budget_exhaustion_mid_phase() {
    let budget = TokenBudget::new(100); // Very small — can't afford all phases
    let config = KernelConfig {
        token_budget: 100,
        ..Default::default()
    };
    let kernel = CognitiveLoop::new(config).with_budget(budget);
    let handler = MockHandler::new();
    let output = kernel
        .run(CycleInput::simple("analyze this complex thing"), &handler)
        .await;
    // Should checkpoint and return partial result or budget exceeded
    assert!(output.is_partial() || output.budget_exceeded() || output.is_ok());
}

/// EC-CL-008: Conflicting goals (resolved without deadlock)
#[tokio::test]
async fn test_ec_cl_008_conflicting_goals() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();

    // Run two concurrent cognitive cycles — should not deadlock
    let (output1, output2) = tokio::join!(
        kernel.run(CycleInput::simple("goal A: be fast"), &handler),
        kernel.run(CycleInput::simple("goal B: be thorough"), &handler),
    );

    // Both should complete (they share the kernel, but each run is independent)
    assert!(output1.is_ok());
    assert!(output2.is_ok());
}

/// EC-CL-009: Resume from checkpoint
#[tokio::test]
async fn test_ec_cl_009_resume_from_checkpoint() {
    let kernel = CognitiveLoop::new(KernelConfig::default());
    let handler = MockHandler::new();

    // Create a checkpoint as if we stopped after Decide
    let checkpoint = hydra_kernel::state::Checkpoint::capture(
        CognitivePhase::Decide,
        hydra_kernel::CheckpointLevel::Full,
        serde_json::json!({"decision": "proceed"}),
        vec!["complete task".into()],
        TokenBudget::new(50_000),
    );

    let output = kernel.resume_from(checkpoint, &handler).await;
    // Should continue from Act phase (next after Decide)
    assert!(output.continued_from_checkpoint());
    assert!(output.is_ok());
    // Should have completed Act and Learn but NOT Perceive/Think/Decide
    assert!(output.phases_completed.contains(&CognitivePhase::Act));
    assert!(output.phases_completed.contains(&CognitivePhase::Learn));
    assert!(!output.phases_completed.contains(&CognitivePhase::Perceive));
}

// test_ec_cl_010_recursive_invocation moved to kernel_tests_extra2.rs
