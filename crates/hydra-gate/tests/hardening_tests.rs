//! Category 1: Unit Gap Fill — hydra-gate edge cases.

use hydra_core::*;
use hydra_gate::*;

// === Risk score boundary cases ===

#[test]
fn test_risk_score_zero() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::Read, "src/main.rs");
    let context = ActionContext {
        in_sandbox: true,
        has_backup: true,
        ..Default::default()
    };
    let assessment = assessor.assess_risk_fast(&action, &context);
    assert!(RiskAssessor::risk_score(&assessment) < 0.5);
}

#[test]
fn test_risk_score_high_for_system() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::System, "/etc/passwd");
    let context = ActionContext::default();
    let assessment = assessor.assess_risk_fast(&action, &context);
    assert!(RiskAssessor::risk_score(&assessment) > 0.5);
}

#[test]
fn test_risk_score_shell_execute() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::ShellExecute, "rm -rf /");
    let context = ActionContext::default();
    let assessment = assessor.assess_risk_fast(&action, &context);
    assert!(RiskAssessor::risk_score(&assessment) > 0.5);
}

// === Gate decision variants ===

#[test]
fn test_gate_all_action_types() {
    let gate = ExecutionGate::default();
    let action_types = vec![
        ActionType::Read,
        ActionType::Write,
        ActionType::Execute,
        ActionType::FileCreate,
        ActionType::FileDelete,
        ActionType::ShellExecute,
        ActionType::GitOperation,
    ];
    for at in action_types {
        let action = Action::new(at, "test/target");
        let context = ActionContext {
            in_sandbox: true,
            ..Default::default()
        };
        let (assessment, duration) = gate.assess_risk_fast_timed(&action, &context);
        assert!(duration.as_millis() < 1000, "risk assessment took too long");
        let _ = RiskAssessor::risk_score(&assessment);
    }
}

// === Gate decision predicates ===

#[test]
fn test_gate_decision_predicates() {
    let auto = GateDecision::AutoApprove { risk_score: 0.1 };
    assert!(auto.is_approved());
    assert!(!auto.is_blocked());
    assert!(!auto.needs_approval());
    assert_eq!(auto.risk_score(), 0.1);

    let block = GateDecision::Block {
        risk_score: 0.95,
        reason: "dangerous".into(),
    };
    assert!(block.is_blocked());
    assert!(!block.is_approved());

    let approval = GateDecision::RequireApproval {
        risk_score: 0.7,
        reason: "risky".into(),
    };
    assert!(approval.needs_approval());

    let timeout = GateDecision::TimedOut { used_default: true };
    assert!(timeout.timed_out());
    assert!(timeout.used_default());
}

// === Boundary enforcement ===

#[test]
fn test_boundary_system_paths_blocked() {
    let enforcer = BoundaryEnforcer::new();
    let paths = vec![
        "/etc/passwd",
        "/System/Library",
        "~/.ssh/id_rsa",
        "/usr/bin/sudo",
    ];
    for path in paths {
        match enforcer.check(path) {
            BoundaryResult::Blocked(v) => {
                assert!(!v.reason.is_empty());
                assert!(!v.rule_name.is_empty());
            }
            BoundaryResult::Allowed => panic!("{} should be blocked", path),
        }
    }
}

#[test]
fn test_boundary_safe_paths_allowed() {
    let enforcer = BoundaryEnforcer::new();
    let paths = vec!["src/main.rs", "/home/user/project/file.txt", "/tmp/test"];
    for path in paths {
        match enforcer.check(path) {
            BoundaryResult::Allowed => {}
            BoundaryResult::Blocked(v) => panic!("{} should be allowed: {}", path, v.reason),
        }
    }
}

#[test]
fn test_boundary_custom_path() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_blocked_path("/custom/blocked");
    match enforcer.check("/custom/blocked/file.txt") {
        BoundaryResult::Blocked(_) => {}
        BoundaryResult::Allowed => panic!("custom path should be blocked"),
    }
}

// === Kill switch ===

#[test]
fn test_kill_switch_halt_record() {
    let ks = KillSwitch::new();
    let record = ks.instant_halt("test halt", "test_user");
    assert!(ks.is_halted());
    assert_eq!(record.reason, "test halt");
    assert_eq!(record.halted_by, "test_user");

    ks.resume();
    assert!(!ks.is_halted());
}

// === Audit chain ===

#[test]
fn test_audit_chain_verification() {
    let gate = ExecutionGate::default();
    // Gate starts with empty audit log
    assert!(gate.verify_audit_chain());
}

// === Gate config ===

#[test]
fn test_gate_config_default_thresholds() {
    let config = GateConfig::default();
    assert!(config.auto_approve_below < config.notify_below);
    assert!(config.notify_below < config.require_approval_below);
    assert!(config.require_approval_below <= config.block_above);
}

// === Shadow simulation ===

#[test]
fn test_shadow_sim_skip() {
    let result = SimResult::skip();
    assert!(!result.simulated);
    assert!(result.is_safe());
}

// === Security layers ===

#[test]
fn test_sanitize_output_redacts_secrets() {
    let input = "API_KEY=sk-1234567890 and PASSWORD=mysecret123";
    let sanitized = security_layers::sanitize_for_output(input);
    assert!(!sanitized.contains("sk-1234567890"));
    assert!(!sanitized.contains("mysecret123"));
}

#[test]
fn test_perimeter_config_default_domains() {
    let config = security_layers::PerimeterConfig::new();
    assert!(config.allowed_domains.contains("github.com"));
    assert!(config.allowed_domains.contains("crates.io"));
}
