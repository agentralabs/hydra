use hydra_core::types::{Action, ActionType, RiskLevel};

use crate::challenge::ChallengeManager;
use crate::gate::{ExecutionGate, GateConfig, GateDecision};
use crate::risk::{ActionContext, RiskAssessor};

use super::tests::{safe_context, safe_read_action};

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
    let action = super::tests::safe_read_action();
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
