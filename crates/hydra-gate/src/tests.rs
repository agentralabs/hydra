use hydra_core::types::{Action, ActionType, RiskLevel};

use crate::gate::{ExecutionGate, GateConfig, GateDecision};
use crate::risk::ActionContext;

// ── Helpers ──

pub(crate) fn safe_read_action() -> Action {
    Action::new(ActionType::Read, "src/main.rs")
}

pub(crate) fn safe_context() -> ActionContext {
    ActionContext {
        target_path: Some("src/main.rs".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    }
}

pub(crate) fn high_risk_action() -> Action {
    // ShellExecute outside sandbox with no backup → high risk
    Action::new(ActionType::ShellExecute, "deploy production")
}

pub(crate) fn high_risk_context() -> ActionContext {
    ActionContext {
        target_path: Some("deploy".into()),
        is_hydra_internal: false,
        in_sandbox: false,
        has_backup: false,
    }
}

pub(crate) fn medium_risk_action() -> Action {
    Action::new(ActionType::Write, "config.toml")
}

pub(crate) fn medium_risk_context() -> ActionContext {
    ActionContext {
        target_path: Some("config.toml".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    }
}

pub(crate) fn critical_action() -> Action {
    // Self-modification → forced critical
    Action::new(ActionType::FileModify, "hydra-gate/src/gate.rs")
}

pub(crate) fn critical_context() -> ActionContext {
    ActionContext {
        target_path: Some("hydra-gate/src/gate.rs".into()),
        is_hydra_internal: true,
        in_sandbox: true,
        has_backup: false,
    }
}

// ── ExecutionGate Tests ──

#[tokio::test]
async fn gate_auto_approves_low_risk() {
    let gate = ExecutionGate::default();
    let decision = gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    assert!(
        matches!(decision, GateDecision::AutoApprove { risk_score } if risk_score < 0.3),
        "Low-risk read should auto-approve, got: {:?}",
        decision
    );
    assert!(decision.is_approved());
    assert!(!decision.is_blocked());
    assert!(!decision.needs_approval());
}

#[tokio::test]
async fn gate_notify_only_for_medium_risk() {
    let gate = ExecutionGate::default();
    let action = medium_risk_action();
    let ctx = medium_risk_context();
    let decision = gate.evaluate(&action, &ctx, None).await;
    // Write to config.toml in sandbox: action_type Write=0.3 * 0.6 = 0.18, path_risk=0, sandbox ok
    // This should land in NotifyOnly or AutoApprove range
    // The exact score depends on factors; verify it's approved
    assert!(
        decision.is_approved(),
        "Medium-risk write in sandbox should be approved (auto or notify), got: {:?}",
        decision
    );
}

#[tokio::test]
async fn gate_requires_approval_for_high_risk() {
    let gate = ExecutionGate::default();
    let action = high_risk_action();
    let ctx = high_risk_context();
    let decision = gate.evaluate(&action, &ctx, None).await;
    // ShellExecute=0.7 * 0.6 = 0.42, no sandbox +0.15, irreversible +0.1 = 0.67
    assert!(
        decision.needs_approval() || decision.is_blocked(),
        "High-risk shell execute outside sandbox should require approval or block, got: {:?}",
        decision
    );
}

#[tokio::test]
async fn gate_blocks_critical_risk() {
    let gate = ExecutionGate::default();
    let action = critical_action();
    let ctx = critical_context();
    let decision = gate.evaluate(&action, &ctx, None).await;
    // is_hydra_internal forces risk_score = 0.95 → block
    assert!(
        decision.is_blocked(),
        "Critical self-modification should be blocked, got: {:?}",
        decision
    );
}

#[tokio::test]
async fn gate_kill_switch_halts_all() {
    let gate = ExecutionGate::default();
    gate.kill_switch().instant_halt("emergency test", "test_suite");
    let decision = gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    assert!(
        matches!(decision, GateDecision::Halted { .. }),
        "Kill switch should halt even safe actions, got: {:?}",
        decision
    );
    assert!(decision.is_blocked());
    assert!(decision.aborted());
}

#[tokio::test]
async fn gate_boundary_violation_blocks() {
    let gate = ExecutionGate::default();
    let action = Action::new(ActionType::Read, "/etc/passwd");
    let ctx = safe_context();
    let decision = gate.evaluate(&action, &ctx, None).await;
    assert!(
        decision.is_blocked(),
        "Boundary violation (/etc/) should block, got: {:?}",
        decision
    );
}

#[tokio::test]
async fn gate_audit_chain_integrity() {
    let gate = ExecutionGate::default();

    // Run several evaluations to build an audit chain
    gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    gate.evaluate(&medium_risk_action(), &medium_risk_context(), None).await;
    gate.evaluate(&safe_read_action(), &safe_context(), None).await;

    let log = gate.audit_log();
    assert!(log.len() >= 3, "Should have at least 3 audit entries, got {}", log.len());

    // Verify chain integrity
    assert!(
        gate.verify_audit_chain(),
        "Audit chain should be tamper-evident and valid"
    );

    // Verify sequence numbers are monotonic
    for (i, entry) in log.iter().enumerate() {
        assert_eq!(entry.sequence, i as u64, "Sequence should match index");
    }

    // First entry should have no previous hash
    assert!(log[0].previous_hash.is_none(), "First entry has no previous hash");

    // Subsequent entries should chain to previous
    for i in 1..log.len() {
        assert!(
            log[i].previous_hash.is_some(),
            "Entry {} should have a previous hash",
            i
        );
        assert_eq!(
            log[i].previous_hash.as_ref().unwrap(),
            &log[i - 1].content_hash,
            "Entry {} prev_hash should match entry {} content_hash",
            i,
            i - 1
        );
    }
}

#[tokio::test]
async fn gate_batch_evaluation() {
    let gate = ExecutionGate::default();
    let actions = vec![
        safe_read_action(),
        medium_risk_action(),
        critical_action(),
    ];
    let ctx = ActionContext {
        target_path: None,
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    };

    let batch = gate.evaluate_batch(&actions, &ctx, None).await;
    assert_eq!(batch.decisions.len(), 3, "Batch should have 3 decisions");

    // First action (safe read) should be approved
    let (idx0, ref dec0) = batch.decisions[0];
    assert_eq!(idx0, 0);
    assert!(dec0.is_approved(), "Safe read should be approved in batch");

    // Third action targets hydra-gate/src → boundary violation → blocked
    let (idx2, ref dec2) = batch.decisions[2];
    assert_eq!(idx2, 2);
    assert!(dec2.is_blocked(), "Critical action should be blocked in batch");

    // Test needs_approval_for helper
    assert!(!batch.needs_approval_for(0), "Safe read should not need approval");
}

#[tokio::test]
async fn gate_decision_properties() {
    // Test GateDecision helper methods
    let auto = GateDecision::AutoApprove { risk_score: 0.1 };
    assert!(auto.is_approved());
    assert!(!auto.is_blocked());
    assert!(!auto.needs_approval());
    assert!(!auto.timed_out());
    assert!(!auto.aborted());
    assert_eq!(auto.risk_score(), 0.1);
    assert_eq!(auto.decision_name(), "auto_approve");

    let notify = GateDecision::NotifyOnly {
        risk_score: 0.35,
        message: "test".into(),
    };
    assert!(notify.is_approved());
    assert_eq!(notify.decision_name(), "notify_only");

    let require = GateDecision::RequireApproval {
        risk_score: 0.6,
        reason: "test".into(),
    };
    assert!(require.needs_approval());
    assert!(!require.is_approved());
    assert_eq!(require.decision_name(), "require_approval");

    let block = GateDecision::Block {
        risk_score: 0.95,
        reason: "test".into(),
    };
    assert!(block.is_blocked());
    assert_eq!(block.risk_level(), RiskLevel::Critical);
    assert_eq!(block.decision_name(), "block");

    let timeout = GateDecision::TimedOut { used_default: true };
    assert!(timeout.timed_out());
    assert!(timeout.used_default());
    assert_eq!(timeout.risk_score(), 0.0);

    let aborted = GateDecision::Aborted {
        reason: "test".into(),
    };
    assert!(aborted.aborted());

    let halted = GateDecision::Halted {
        reason: "test".into(),
    };
    assert!(halted.is_blocked());
    assert!(halted.aborted());
    assert_eq!(halted.decision_name(), "halted");
}
