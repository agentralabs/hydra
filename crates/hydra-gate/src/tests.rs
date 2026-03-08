use hydra_core::types::{Action, ActionType, RiskLevel};

use crate::boundary::{BoundaryEnforcer, BoundaryResult};
use crate::challenge::ChallengeManager;
use crate::gate::{ExecutionGate, GateConfig, GateDecision};
use crate::kill_switch::KillSwitch;
use crate::risk::{ActionContext, RiskAssessor};

// ── Helpers ──

fn safe_read_action() -> Action {
    Action::new(ActionType::Read, "src/main.rs")
}

fn safe_context() -> ActionContext {
    ActionContext {
        target_path: Some("src/main.rs".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    }
}

fn high_risk_action() -> Action {
    // ShellExecute outside sandbox with no backup → high risk
    Action::new(ActionType::ShellExecute, "deploy production")
}

fn high_risk_context() -> ActionContext {
    ActionContext {
        target_path: Some("deploy".into()),
        is_hydra_internal: false,
        in_sandbox: false,
        has_backup: false,
    }
}

fn medium_risk_action() -> Action {
    Action::new(ActionType::Write, "config.toml")
}

fn medium_risk_context() -> ActionContext {
    ActionContext {
        target_path: Some("config.toml".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    }
}

fn critical_action() -> Action {
    // Self-modification → forced critical
    Action::new(ActionType::FileModify, "hydra-gate/src/gate.rs")
}

fn critical_context() -> ActionContext {
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

// ── BoundaryEnforcer Tests ──

#[test]
fn boundary_blocks_system_paths() {
    let enforcer = BoundaryEnforcer::new();
    let blocked_paths = vec![
        "/System/Library/config",
        "~/.ssh/id_rsa",
        ".ssh/authorized_keys",
        "~/.gnupg/keys",
        ".gnupg/pubring.kbx",
        "/etc/shadow",
        "/usr/bin/bash",
        "/usr/sbin/service",
        "/sbin/init",
        "/boot/vmlinuz",
        "/proc/1/status",
        "/sys/class/net",
    ];
    for path in blocked_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Blocked(_)),
            "Path '{}' should be blocked",
            path
        );
    }
}

#[test]
fn boundary_blocks_destructive_commands() {
    let enforcer = BoundaryEnforcer::new();
    assert!(
        matches!(enforcer.check("rm -rf /"), BoundaryResult::Blocked(_)),
        "rm -rf / should be blocked"
    );
}

#[test]
fn boundary_blocks_email_and_payment() {
    let enforcer = BoundaryEnforcer::new();
    assert!(matches!(
        enforcer.check("send_email user@example.com"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("process_payment $500"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_blocks_self_modification() {
    let enforcer = BoundaryEnforcer::new();
    let self_mod_targets = vec![
        "hydra-gate/src/gate.rs",
        "hydra-kernel/src/cognitive_loop.rs",
        "hydra-core/src/types.rs",
    ];
    for target in self_mod_targets {
        assert!(
            matches!(enforcer.check(target), BoundaryResult::Blocked(_)),
            "Self-modification target '{}' should be blocked",
            target
        );
    }
}

#[test]
fn boundary_allows_safe_paths() {
    let enforcer = BoundaryEnforcer::new();
    let safe_paths = vec![
        "src/main.rs",
        "/home/user/project/lib.rs",
        "/tmp/test.txt",
        "Cargo.toml",
        "README.md",
        "/Users/dev/project/app.py",
    ];
    for path in safe_paths {
        assert!(
            matches!(enforcer.check(path), BoundaryResult::Allowed),
            "Path '{}' should be allowed",
            path
        );
    }
}

#[test]
fn boundary_case_insensitive() {
    let enforcer = BoundaryEnforcer::new();
    assert!(matches!(
        enforcer.check("/ETC/PASSWD"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("/SYSTEM/Library"),
        BoundaryResult::Blocked(_)
    ));
    assert!(matches!(
        enforcer.check("SEND_EMAIL"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_custom_blocked_path() {
    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_blocked_path("/custom/secret/");
    assert!(matches!(
        enforcer.check("/custom/secret/data.json"),
        BoundaryResult::Blocked(_)
    ));
    // Original blocks still work
    assert!(matches!(
        enforcer.check("/etc/hosts"),
        BoundaryResult::Blocked(_)
    ));
}

#[test]
fn boundary_check_action_with_hard_boundary() {
    use crate::boundary::HardBoundary;

    let mut enforcer = BoundaryEnforcer::new();
    enforcer.add_boundary(HardBoundary {
        description: "No database drops".into(),
        blocked_actions: vec!["drop_database".into()],
    });

    let result = enforcer.check_action("drop_database", "production_db");
    assert!(
        matches!(result, BoundaryResult::Blocked(_)),
        "Hard boundary should block drop_database action"
    );

    let result = enforcer.check_action("query_database", "production_db");
    assert!(
        matches!(result, BoundaryResult::Allowed),
        "Non-blocked action should be allowed"
    );
}

#[test]
fn boundary_violation_display() {
    let enforcer = BoundaryEnforcer::new();
    if let BoundaryResult::Blocked(violation) = enforcer.check("/etc/passwd") {
        let display = format!("{}", violation);
        assert!(display.contains("blocked_path"), "Display should show rule name");
        assert!(display.contains("/etc/passwd"), "Display should show target");
    } else {
        panic!("/etc/passwd should be blocked");
    }
}

// ── KillSwitch Tests ──

#[test]
fn kill_switch_instant_halt() {
    let ks = KillSwitch::new();
    assert!(!ks.is_halted(), "Should not be halted initially");

    let record = ks.instant_halt("test emergency", "test_user");
    assert!(ks.is_halted(), "Should be halted after instant_halt");
    assert_eq!(record.reason, "test emergency");
    assert_eq!(record.halted_by, "test_user");
}

#[test]
fn kill_switch_resume() {
    let ks = KillSwitch::new();
    ks.instant_halt("test halt", "test_user");
    assert!(ks.is_halted());

    ks.resume();
    assert!(!ks.is_halted(), "Should not be halted after resume");
    assert!(
        ks.halt_reason().is_none(),
        "Halt reason should be cleared after resume"
    );
}

#[test]
fn kill_switch_halt_reason_recorded() {
    let ks = KillSwitch::new();
    assert!(ks.halt_reason().is_none(), "No reason before halt");

    ks.instant_halt("critical bug found", "admin");
    let reason = ks.halt_reason().expect("Should have halt reason");
    assert_eq!(reason.reason, "critical bug found");
    assert_eq!(reason.halted_by, "admin");
}

#[test]
fn kill_switch_concurrent_halt_check() {
    use std::sync::Arc;
    use std::thread;

    let ks = Arc::new(KillSwitch::new());
    let ks_clone = ks.clone();

    // Halt from another thread
    let handle = thread::spawn(move || {
        ks_clone.instant_halt("concurrent halt", "thread");
    });
    handle.join().unwrap();

    // Check from main thread
    assert!(ks.is_halted(), "Halt should be visible across threads");
    let reason = ks.halt_reason().unwrap();
    assert_eq!(reason.reason, "concurrent halt");
}

#[test]
fn kill_switch_multiple_halt_resume_cycles() {
    let ks = KillSwitch::new();

    for i in 0..5 {
        ks.instant_halt(format!("halt {}", i), "cycle_test");
        assert!(ks.is_halted());
        ks.resume();
        assert!(!ks.is_halted());
    }
}

// ── RiskAssessor Tests ──

#[test]
fn risk_read_action_is_low() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::Read, "src/lib.rs");
    let ctx = safe_context();
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let score = RiskAssessor::risk_score(&assessment);
    assert!(
        score < 0.3,
        "Read action on safe path should have low risk, got {}",
        score
    );
    assert!(!assessment.requires_approval);
}

#[test]
fn risk_shell_execute_is_high() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::ShellExecute, "make deploy");
    let ctx = ActionContext {
        target_path: Some("deploy".into()),
        is_hydra_internal: false,
        in_sandbox: false,
        has_backup: false,
    };
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let score = RiskAssessor::risk_score(&assessment);
    assert!(
        score >= 0.5,
        "Shell execute outside sandbox should be high risk, got {}",
        score
    );
    assert!(assessment.requires_approval);
}

#[test]
fn risk_self_modification_is_critical() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::FileModify, "hydra-core/src/types.rs");
    let ctx = ActionContext {
        target_path: Some("hydra-core/src/types.rs".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    };
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let score = RiskAssessor::risk_score(&assessment);
    assert!(
        score >= 0.9,
        "Self-modification should be critical risk, got {}",
        score
    );
    assert_eq!(assessment.level, RiskLevel::Critical);
}

#[test]
fn risk_sensitive_path_elevates_score() {
    let assessor = RiskAssessor::new();
    // Use a Write action on .env so path sensitivity + action type combine
    // to push risk above the None threshold
    let action = Action::new(ActionType::Write, ".env.production");
    let ctx = ActionContext {
        target_path: Some(".env.production".into()),
        is_hydra_internal: false,
        in_sandbox: false, // outside sandbox adds +0.15
        has_backup: false,
    };
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let score = RiskAssessor::risk_score(&assessment);
    // Write=0.3*0.6=0.18, .env path_risk=0.7*0.25=0.175, no sandbox +0.15, irreversible(network)=no
    // Total ~0.505 → Medium → risk_score = 0.55
    assert!(
        score > 0.1,
        "Writing .env outside sandbox should have elevated risk, got {}",
        score
    );
    // Also verify the path sensitivity factor was detected
    let has_target_path_factor = assessment.factors.iter().any(|f| f.name == "target_path");
    assert!(
        has_target_path_factor,
        ".env path should trigger target_path risk factor"
    );
}

#[test]
fn risk_blast_radius_assessment() {
    use crate::risk::BlastRadius;

    // Verify BlastRadius enum variants exist and are distinct
    let local = BlastRadius::Local;
    let project = BlastRadius::Project;
    let system = BlastRadius::System;
    let external = BlastRadius::External;
    let social = BlastRadius::Social;
    let financial = BlastRadius::Financial;

    assert_eq!(local, BlastRadius::Local);
    assert_ne!(local, project);
    assert_ne!(system, external);
    assert_ne!(social, financial);
}

#[test]
fn risk_file_delete_without_backup_is_irreversible() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::FileDelete, "important_data.sql");
    let ctx = ActionContext {
        target_path: Some("important_data.sql".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    };
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let has_irreversible = assessment
        .factors
        .iter()
        .any(|f| f.name == "irreversible");
    assert!(
        has_irreversible,
        "FileDelete without backup should flag irreversible factor"
    );
}

#[test]
fn risk_sandbox_reduces_risk() {
    let assessor = RiskAssessor::new();
    let action = Action::new(ActionType::Execute, "test_script.sh");

    let sandboxed = ActionContext {
        target_path: Some("test_script.sh".into()),
        is_hydra_internal: false,
        in_sandbox: true,
        has_backup: false,
    };
    let unsandboxed = ActionContext {
        target_path: Some("test_script.sh".into()),
        is_hydra_internal: false,
        in_sandbox: false,
        has_backup: false,
    };

    let score_in = RiskAssessor::risk_score(&assessor.assess_risk_fast(&action, &sandboxed));
    let score_out = RiskAssessor::risk_score(&assessor.assess_risk_fast(&action, &unsandboxed));

    assert!(
        score_out >= score_in,
        "Outside sandbox ({}) should be >= inside sandbox ({})",
        score_out,
        score_in
    );
}

#[test]
fn risk_explicit_risk_level_overrides() {
    let assessor = RiskAssessor::new();
    let mut action = Action::new(ActionType::Read, "safe_file.txt");
    action.risk = RiskLevel::High;
    let ctx = safe_context();
    let assessment = assessor.assess_risk_fast(&action, &ctx);
    let score = RiskAssessor::risk_score(&assessment);
    assert!(
        score >= 0.7,
        "Explicitly High risk should override base score, got {}",
        score
    );
}

#[test]
fn risk_fast_assessment_timing() {
    let gate = ExecutionGate::default();
    let action = safe_read_action();
    let ctx = safe_context();

    let (_assessment, duration) = gate.assess_risk_fast_timed(&action, &ctx);
    assert!(
        duration.as_millis() < 50,
        "Fast assessment should be < 50ms, took {}ms",
        duration.as_millis()
    );
}

// ── ChallengeManager Tests ──

#[test]
fn challenge_generate_and_verify() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-1");

    assert!(!challenge.phrase.is_empty(), "Challenge phrase should not be empty");
    assert_eq!(challenge.action_id, "action-1");
    assert!(!challenge.is_expired(), "Fresh challenge should not be expired");

    // Verify with correct phrase
    let phrase = challenge.phrase.clone();
    assert!(
        mgr.validate("action-1", &phrase),
        "Should validate with correct phrase"
    );

    // Challenge is consumed after validation
    assert!(
        !mgr.validate("action-1", &phrase),
        "Challenge should be consumed after successful validation"
    );
}

#[test]
fn challenge_case_insensitive_validation() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-2");
    let phrase_lower = challenge.phrase.to_lowercase();

    assert!(
        mgr.validate("action-2", &phrase_lower),
        "Validation should be case-insensitive"
    );
}

#[test]
fn challenge_wrong_phrase_rejected() {
    let mut mgr = ChallengeManager::new(120);
    mgr.generate("action-3");

    assert!(
        !mgr.validate("action-3", "WRONG PHRASE"),
        "Wrong phrase should be rejected"
    );
}

#[test]
fn challenge_wrong_action_id_rejected() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("action-4");

    assert!(
        !mgr.validate("action-999", &challenge.phrase),
        "Wrong action_id should be rejected"
    );
}

#[test]
fn challenge_expired_rejected() {
    // Use a TTL of 0 seconds so it expires immediately
    let mut mgr = ChallengeManager::new(0);
    let challenge = mgr.generate("action-5");
    let phrase = challenge.phrase.clone();

    // The challenge should be expired (or very close to it)
    // Sleep a tiny bit to ensure expiration
    std::thread::sleep(std::time::Duration::from_millis(10));

    assert!(
        !mgr.validate("action-5", &phrase),
        "Expired challenge should be rejected"
    );
}

#[test]
fn challenge_active_count() {
    let mut mgr = ChallengeManager::new(120);
    assert_eq!(mgr.active_count(), 0);

    mgr.generate("a");
    mgr.generate("b");
    mgr.generate("c");
    assert_eq!(mgr.active_count(), 3);

    // Validate one (consumes it)
    let phrase = mgr.generate("d").phrase.clone();
    mgr.validate("d", &phrase);
    assert_eq!(mgr.active_count(), 3, "Consumed challenge should be removed");
}

#[test]
fn challenge_replace_existing() {
    let mut mgr = ChallengeManager::new(120);
    let first = mgr.generate("action-dup");
    let first_phrase = first.phrase.clone();

    let second = mgr.generate("action-dup");
    let second_phrase = second.phrase.clone();

    // Old phrase should no longer work (replaced)
    if first_phrase != second_phrase {
        assert!(
            !mgr.validate("action-dup", &first_phrase),
            "Old challenge phrase should be replaced"
        );
    }
    // New phrase should work
    assert!(
        mgr.validate("action-dup", &second_phrase),
        "New challenge phrase should validate"
    );
}

#[test]
fn challenge_expire_old_cleanup() {
    let mut mgr = ChallengeManager::new(0);
    mgr.generate("old-1");
    mgr.generate("old-2");
    std::thread::sleep(std::time::Duration::from_millis(10));

    mgr.expire_old();
    assert_eq!(mgr.active_count(), 0, "All expired challenges should be cleaned up");
}

#[test]
fn challenge_phrase_has_two_words() {
    let mut mgr = ChallengeManager::new(120);
    let challenge = mgr.generate("word-check");
    let parts: Vec<&str> = challenge.phrase.split_whitespace().collect();
    assert_eq!(parts.len(), 2, "Challenge phrase should have exactly 2 words: '{}'", challenge.phrase);
}

// ── Integration Tests ──

#[tokio::test]
async fn gate_kill_switch_overrides_everything() {
    let gate = ExecutionGate::default();
    gate.kill_switch().instant_halt("lockdown", "security");

    // Even boundary-violating actions should get Halted, not Block
    let action = Action::new(ActionType::Read, "/etc/passwd");
    let decision = gate.evaluate(&action, &safe_context(), None).await;
    assert!(
        matches!(decision, GateDecision::Halted { .. }),
        "Kill switch should take priority over boundary check"
    );
}

#[tokio::test]
async fn gate_resume_after_kill_switch() {
    let gate = ExecutionGate::default();
    gate.kill_switch().instant_halt("temporary halt", "test");
    assert!(matches!(
        gate.evaluate(&safe_read_action(), &safe_context(), None).await,
        GateDecision::Halted { .. }
    ));

    gate.kill_switch().resume();
    let decision = gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    assert!(
        decision.is_approved(),
        "After resume, safe actions should be approved again"
    );
}

#[tokio::test]
async fn gate_config_update_changes_thresholds() {
    let gate = ExecutionGate::default();

    // With default config, a read is auto-approved
    let decision = gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    assert!(decision.is_approved());

    // Change config to block everything (block_above = 0.0)
    gate.update_config(GateConfig {
        auto_approve_below: 0.0,
        notify_below: 0.0,
        require_approval_below: 0.0,
        block_above: 0.0,
        ..GateConfig::default()
    });

    let decision = gate.evaluate(&safe_read_action(), &safe_context(), None).await;
    assert!(
        decision.is_blocked(),
        "With block_above=0.0, everything should be blocked"
    );
}
