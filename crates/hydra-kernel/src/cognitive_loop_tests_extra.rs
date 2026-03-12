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
