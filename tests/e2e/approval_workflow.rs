//! Category 3: E2E — approval workflow.

use hydra_core::*;
use hydra_gate::*;
use hydra_runtime::*;

#[test]
fn test_high_risk_requires_approval() {
    let gate = ExecutionGate::default();
    let action = Action::new(ActionType::FileDelete, "/important/data");
    let context = ActionContext::default();
    let (assessment, _) = gate.assess_risk_fast_timed(&action, &context);
    let score = RiskAssessor::risk_score(&assessment);
    // File deletion of important path should be medium-high risk
    assert!(score > 0.3);
}

#[test]
fn test_read_action_auto_approves() {
    let action = Action::new(ActionType::Read, "src/main.rs");
    let context = ActionContext { in_sandbox: true, has_backup: true, ..Default::default() };
    let assessor = RiskAssessor::new();
    let assessment = assessor.assess_risk_fast(&action, &context);
    let score = RiskAssessor::risk_score(&assessment);
    // Read in sandbox should be low risk
    assert!(score < 0.5);
}

#[test]
fn test_approval_timeout_denies() {
    let mgr = ApprovalManager::new(std::time::Duration::from_millis(1));
    let (req, rx) = mgr.request_approval("run1", "dangerous action", None, 0.9, "high risk");
    assert!(mgr.is_pending(&req.id));

    // Wait for timeout
    std::thread::sleep(std::time::Duration::from_millis(50));
    let result = mgr.wait_for_approval(&req.id, rx);
    assert!(result.is_err()); // should timeout
}

#[test]
fn test_approval_modify_and_proceed() {
    let mgr = ApprovalManager::with_default_timeout();
    let (req, _rx) = mgr.request_approval("run1", "delete files", None, 0.7, "moderate risk");

    mgr.submit_decision(&req.id, ApprovalDecision::Modified {
        new_action: "move to trash instead".into(),
    }).unwrap();

    assert_eq!(mgr.get_status(&req.id), Some(ApprovalStatus::Modified));
}

#[test]
fn test_boundary_blocks_before_gate() {
    let gate = ExecutionGate::default();
    let enforcer = gate.boundary();
    // System paths should be blocked at boundary level
    match enforcer.check("/etc/shadow") {
        BoundaryResult::Blocked(_) => {} // expected
        BoundaryResult::Allowed => panic!("/etc/shadow should be blocked"),
    }
}
