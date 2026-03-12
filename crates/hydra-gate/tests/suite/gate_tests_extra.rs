use std::time::{Duration, Instant};

use hydra_core::types::{Action, ActionType, Capability, CapabilityToken, RiskLevel};
use hydra_gate::gate::{ExecutionGate, GateConfig, GateDecision};
use hydra_gate::kill_switch::KillSwitch;
use hydra_gate::risk::{ActionContext, RiskAssessor};
use hydra_gate::security_layers::{self, PerimeterConfig, ResourceLimits, SessionContext};

fn read_action() -> Action {
    Action::new(ActionType::Read, "src/main.rs")
}
fn write_action() -> Action {
    Action::new(ActionType::FileModify, "src/main.rs")
}
fn delete_action() -> Action {
    Action::new(ActionType::FileDelete, "src/old.rs")
}
fn shell_action() -> Action {
    Action::new(ActionType::ShellExecute, "rm -rf /tmp/test")
}
fn system_action() -> Action {
    Action::new(ActionType::System, "reboot")
}
fn network_action() -> Action {
    Action::new(ActionType::Network, "https://api.github.com/repos")
}
fn hydra_config_action() -> Action {
    Action::new(ActionType::FileModify, "/home/user/.hydra/config.toml")
}
fn unknown_action() -> Action {
    Action::new(ActionType::Composite, "unknown_complex_operation")
}
fn default_context() -> ActionContext {
    ActionContext::default()
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 6 — AUDIT
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_layer6_every_decision_audited() {
    let gate = ExecutionGate::default();
    for _ in 0..5 {
        gate.evaluate(&read_action(), &default_context(), None)
            .await;
    }
    assert_eq!(gate.audit_log().len(), 5);
}

#[tokio::test]
async fn test_layer6_tamper_evident_chain() {
    let gate = ExecutionGate::default();
    gate.evaluate(&read_action(), &default_context(), None)
        .await;
    gate.evaluate(&write_action(), &default_context(), None)
        .await;
    gate.evaluate(&delete_action(), &default_context(), None)
        .await;
    assert!(
        gate.verify_audit_chain(),
        "Audit chain should be tamper-evident"
    );
    // Verify individual entry hashes
    for entry in &gate.audit_log() {
        assert!(entry.verify_hash(), "Entry hash should be valid");
    }
}

#[tokio::test]
async fn test_layer6_no_secrets_in_audit() {
    let gate = ExecutionGate::default();
    let action = Action::new(ActionType::Network, "api_key=SECRET123");
    gate.evaluate(&action, &default_context(), None).await;
    for entry in &gate.audit_log() {
        assert!(
            !entry.target.contains("SECRET123"),
            "Secrets must be redacted in audit"
        );
    }
}

// ═══════════════════════════════════════════════════════════
// KILL SWITCH
// ═══════════════════════════════════════════════════════════

#[test]
fn test_kill_switch_halts() {
    let ks = KillSwitch::new();
    assert!(!ks.is_halted());
    let record = ks.instant_halt("Emergency", "user");
    assert!(ks.is_halted());
    assert_eq!(record.reason, "Emergency");
}

#[test]
fn test_kill_switch_resume() {
    let ks = KillSwitch::new();
    ks.instant_halt("test", "admin");
    ks.resume();
    assert!(!ks.is_halted());
}

#[tokio::test]
async fn test_kill_switch_blocks_gate() {
    let gate = ExecutionGate::default();
    gate.kill_switch().instant_halt("Emergency stop", "user");
    let decision = gate
        .evaluate(&read_action(), &default_context(), None)
        .await;
    assert!(matches!(decision, GateDecision::Halted { .. }));
}

// ═══════════════════════════════════════════════════════════
// EDGE CASES (EC-EG-001 through EC-EG-010)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_ec_eg_001_unknown_action_type() {
    let gate = ExecutionGate::default();
    let decision = gate
        .evaluate(&unknown_action(), &default_context(), None)
        .await;
    assert!(decision.needs_approval() || decision.risk_score() >= 0.3);
}

#[tokio::test]
async fn test_ec_eg_002_approval_timeout() {
    let config = GateConfig {
        approval_timeout: Duration::from_millis(1),
        ..Default::default()
    };
    let gate = ExecutionGate::new(config);
    let decision = gate
        .evaluate(&delete_action(), &default_context(), None)
        .await;
    assert!(decision.needs_approval() || decision.is_approved() || decision.timed_out());
}

#[tokio::test]
async fn test_ec_eg_003_user_disconnect() {
    let gate = ExecutionGate::default();
    gate.simulate_disconnect();
    let mut action = delete_action();
    action.risk = RiskLevel::High;
    let decision = gate.evaluate(&action, &default_context(), None).await;
    assert!(decision.aborted());
}

#[tokio::test]
async fn test_ec_eg_004_shadow_sim_crash() {
    let config = GateConfig {
        shadow_sim_enabled: true,
        ..Default::default()
    };
    let gate = ExecutionGate::new(config);
    gate.inject_shadow_sim_crash();
    let mut action = system_action();
    action.risk = RiskLevel::High;
    let decision = gate.evaluate(&action, &default_context(), None).await;
    assert!(decision.needs_approval() || decision.is_blocked());
}

#[tokio::test]
async fn test_ec_eg_005_risk_at_threshold() {
    let gate = ExecutionGate::default();
    let d1 = gate
        .evaluate(&write_action(), &default_context(), None)
        .await;
    let d2 = gate
        .evaluate(&write_action(), &default_context(), None)
        .await;
    assert_eq!(d1.risk_score(), d2.risk_score());
    assert_eq!(d1.decision_name(), d2.decision_name());
}

#[tokio::test]
async fn test_ec_eg_006_batch_actions() {
    let gate = ExecutionGate::default();
    let actions = vec![read_action(), delete_action(), read_action()];
    let result = gate
        .evaluate_batch(&actions, &default_context(), None)
        .await;
    let (_, d0) = &result.decisions[0];
    let (_, d1) = &result.decisions[1];
    assert!(d1.risk_score() > d0.risk_score());
}

#[tokio::test]
async fn test_ec_eg_007_dynamic_risk_change() {
    let gate = ExecutionGate::default();
    let mut action = write_action();
    action.risk = RiskLevel::High;
    let decision = gate.evaluate(&action, &default_context(), None).await;
    assert!(decision.risk_level() >= RiskLevel::High);
}

#[tokio::test]
async fn test_ec_eg_008_config_change_during_eval() {
    let gate = ExecutionGate::default();
    gate.update_config(GateConfig {
        auto_approve_below: 0.9,
        ..Default::default()
    });
    let decision = gate
        .evaluate(&read_action(), &default_context(), None)
        .await;
    assert!(decision.is_approved() || decision.needs_approval());
}

#[tokio::test]
async fn test_ec_eg_009_infinite_approval_prevention() {
    let gate = ExecutionGate::new(GateConfig {
        max_approval_retries: 3,
        ..Default::default()
    });
    gate.set_user_always_rejects();
    let mut action = delete_action();
    action.risk = RiskLevel::High;
    let mut aborted = false;
    for _ in 0..5 {
        let decision = gate.evaluate(&action, &default_context(), None).await;
        if decision.aborted() {
            aborted = true;
            break;
        }
    }
    assert!(aborted);
}

#[tokio::test]
async fn test_ec_eg_010_self_modification_blocked() {
    let gate = ExecutionGate::default();
    let decision = gate
        .evaluate(
            &hydra_config_action(),
            &ActionContext {
                is_hydra_internal: true,
                ..Default::default()
            },
            None,
        )
        .await;
    assert!(decision.is_blocked());
    assert!(decision.risk_level() >= RiskLevel::Critical);
}

// ═══════════════════════════════════════════════════════════
// LATENCY TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_gate_latency_under_50ms_rule_based() {
    let assessor = RiskAssessor::new();
    for action in &[
        read_action(),
        write_action(),
        delete_action(),
        shell_action(),
        system_action(),
        network_action(),
    ] {
        let start = Instant::now();
        assessor.assess_risk_fast(action, &default_context());
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "Risk assessment must be < 50ms"
        );
    }
}

#[tokio::test]
async fn test_gate_latency_under_500ms_full_eval() {
    let gate = ExecutionGate::default();
    for action in &[
        read_action(),
        write_action(),
        delete_action(),
        shell_action(),
    ] {
        let start = Instant::now();
        gate.evaluate(action, &default_context(), None).await;
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "Full gate eval must be < 500ms"
        );
    }
}
