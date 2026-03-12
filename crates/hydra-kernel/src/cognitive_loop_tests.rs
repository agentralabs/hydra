use super::*;
use crate::config::KernelConfig;
use serde_json::json;

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
