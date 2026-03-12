use hydra_core::error::HydraError;
use hydra_core::*;
use std::time::Duration;
use uuid::Uuid;

use super::helpers::{make_compiled_intent, make_receipt};

// ═══════════════════════════════════════════════════════════
// INTENT & ACTION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_raw_intent_new() {
    let intent = Intent::new("create a file", IntentSource::Cli);
    assert_eq!(intent.text, "create a file");
    assert_eq!(intent.metadata.source, IntentSource::Cli);
    assert!(intent.metadata.session_id.is_none());
}

#[test]
fn test_action_new() {
    let action = Action::new(ActionType::FileCreate, "/tmp/test.rs");
    assert_eq!(action.action_type, ActionType::FileCreate);
    assert_eq!(action.target, "/tmp/test.rs");
    assert_eq!(action.risk, RiskLevel::None);
}

#[test]
fn test_action_type_all_variants() {
    let types = [
        ActionType::Read,
        ActionType::Write,
        ActionType::Execute,
        ActionType::Network,
        ActionType::System,
        ActionType::FileCreate,
        ActionType::FileModify,
        ActionType::FileDelete,
        ActionType::ShellExecute,
        ActionType::GitOperation,
        ActionType::ApiCall,
        ActionType::SisterCall,
        ActionType::Composite,
    ];
    assert_eq!(types.len(), 13);
}

#[test]
fn test_compiled_intent_high_confidence() {
    let intent = make_compiled_intent(0.9, 1, vec![ActionType::FileCreate]);
    assert!(intent.is_high_confidence());
}

#[test]
fn test_compiled_intent_low_confidence() {
    let intent = make_compiled_intent(0.5, 1, vec![ActionType::FileCreate]);
    assert!(!intent.is_high_confidence());
}

#[test]
fn test_compiled_intent_multi_step() {
    let intent = make_compiled_intent(0.9, 3, vec![ActionType::FileCreate]);
    assert!(intent.is_multi_step());
}

#[test]
fn test_compiled_intent_single_step() {
    let intent = make_compiled_intent(0.9, 1, vec![ActionType::FileCreate]);
    assert!(!intent.is_multi_step());
}

#[test]
fn test_compiled_intent_destructive() {
    let intent = make_compiled_intent(0.9, 1, vec![ActionType::FileDelete, ActionType::System]);
    assert!(intent.has_destructive_actions());
}

#[test]
fn test_compiled_intent_non_destructive() {
    let intent = make_compiled_intent(0.9, 1, vec![ActionType::Read, ActionType::ApiCall]);
    assert!(!intent.has_destructive_actions());
}

#[test]
fn test_compiled_intent_tokens_used() {
    let intent = make_compiled_intent(0.9, 1, vec![ActionType::Read]);
    assert_eq!(intent.tokens_used, 0);
}

#[test]
fn test_compiled_intent_action_types() {
    let intent = make_compiled_intent(0.9, 2, vec![ActionType::Read, ActionType::Write]);
    let types = intent.action_types();
    assert_eq!(types.len(), 2);
    assert_eq!(*types[0], ActionType::Read);
    assert_eq!(*types[1], ActionType::Write);
}

#[test]
fn test_action_result_fields() {
    let result = ActionResult {
        success: true,
        output: serde_json::json!({"file": "created"}),
        side_effects: vec![SideEffect {
            description: "File written to disk".into(),
            reversible: true,
        }],
        duration: Duration::from_millis(150),
    };
    assert!(result.success);
    assert_eq!(result.side_effects.len(), 1);
    assert!(result.side_effects[0].reversible);
}

// ═══════════════════════════════════════════════════════════
// TOKEN BUDGET TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_token_budget_new() {
    let budget = TokenBudget::new(100_000);
    assert_eq!(budget.total, 100_000);
    assert_eq!(budget.remaining, 100_000);
    assert!(!budget.conservation_mode);
}

#[test]
fn test_token_budget_can_afford() {
    let budget = TokenBudget::new(100_000);
    assert!(budget.can_afford(50_000));
    assert!(budget.can_afford(100_000));
    assert!(!budget.can_afford(100_001));
}

#[test]
fn test_token_budget_record_usage() {
    let mut budget = TokenBudget::new(100_000);
    budget.record_usage(30_000);
    assert_eq!(budget.remaining, 70_000);
    assert_eq!(budget.used(), 30_000);
    assert!(budget.can_afford(70_000));
    assert!(!budget.can_afford(70_001));
}

#[test]
fn test_token_budget_conservation_mode_activates() {
    let mut budget = TokenBudget::new(100_000);
    assert!(!budget.conservation_mode);
    budget.record_usage(76_000); // 24% remaining
    assert!(budget.conservation_mode);
}

#[test]
fn test_token_budget_utilization() {
    let mut budget = TokenBudget::new(100_000);
    assert!((budget.utilization() - 0.0).abs() < f64::EPSILON);
    budget.record_usage(50_000);
    assert!((budget.utilization() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_token_budget_zero_total() {
    let budget = TokenBudget::new(0);
    assert_eq!(budget.utilization(), 0.0);
    assert!(!budget.can_afford(1));
    assert!(budget.conservation_mode); // 0 remaining triggers conservation
}

#[test]
fn test_token_budget_per_phase() {
    let budget = TokenBudget::new(100_000);
    assert!(budget.per_phase.contains_key(&CognitivePhase::Perceive));
    assert!(budget.per_phase.contains_key(&CognitivePhase::Think));
    assert!(budget.per_phase.contains_key(&CognitivePhase::Decide));
    assert!(budget.per_phase.contains_key(&CognitivePhase::Act));
    assert!(budget.per_phase.contains_key(&CognitivePhase::Learn));
}

#[test]
fn test_token_metrics_defaults() {
    let metrics = TokenMetrics::default();
    assert_eq!(metrics.used, 0);
    assert_eq!(metrics.cached_hits, 0);
    assert_eq!(metrics.llm_calls, 0);
    assert_eq!(metrics.efficiency, 0.0);
}

// ═══════════════════════════════════════════════════════════
// RISK ASSESSMENT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_risk_assessment_needs_approval_high() {
    let assessment = RiskAssessment {
        level: RiskLevel::High,
        factors: vec![],
        mitigations: vec![],
        requires_approval: false,
    };
    assert!(assessment.needs_approval());
}

#[test]
fn test_risk_assessment_needs_approval_explicit() {
    let assessment = RiskAssessment {
        level: RiskLevel::Low,
        factors: vec![],
        mitigations: vec![],
        requires_approval: true,
    };
    assert!(assessment.needs_approval());
}

#[test]
fn test_risk_assessment_no_approval_needed() {
    let assessment = RiskAssessment {
        level: RiskLevel::Low,
        factors: vec![],
        mitigations: vec![],
        requires_approval: false,
    };
    assert!(!assessment.needs_approval());
}

#[test]
fn test_risk_level_ordering() {
    assert!(RiskLevel::None < RiskLevel::Low);
    assert!(RiskLevel::Low < RiskLevel::Medium);
    assert!(RiskLevel::Medium < RiskLevel::High);
    assert!(RiskLevel::High < RiskLevel::Critical);
}

// ═══════════════════════════════════════════════════════════
// RECEIPT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_receipt_chain_valid_first() {
    let receipt = make_receipt(0, None);
    assert!(receipt.is_chain_valid(None));
}

#[test]
fn test_receipt_chain_valid_subsequent() {
    let first = make_receipt(0, None);
    let second = make_receipt(1, Some(first.content_hash.clone()));
    assert!(second.is_chain_valid(Some(&first)));
}

#[test]
fn test_receipt_chain_invalid_wrong_hash() {
    let first = make_receipt(0, None);
    let second = make_receipt(1, Some("wrong_hash".to_string()));
    assert!(!second.is_chain_valid(Some(&first)));
}

#[test]
fn test_receipt_chain_invalid_missing_previous() {
    let second = make_receipt(1, Some("hash0".to_string()));
    assert!(!second.is_chain_valid(None));
}

#[test]
fn test_receipt_id_unique() {
    let id1 = ReceiptId::new();
    let id2 = ReceiptId::new();
    assert_ne!(id1, id2);
}

// ═══════════════════════════════════════════════════════════
// ICON STATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_all_8_icon_states() {
    let states = [
        IconState::Idle,
        IconState::Listening,
        IconState::Working,
        IconState::NeedsAttention,
        IconState::ApprovalNeeded,
        IconState::Success,
        IconState::Error,
        IconState::Offline,
    ];
    assert_eq!(states.len(), 8);
    for state in &states {
        assert!(!state.animation_description().is_empty());
    }
}

#[test]
fn test_icon_state_transient() {
    assert!(IconState::Success.is_transient());
    assert!(!IconState::Idle.is_transient());
    assert!(!IconState::Working.is_transient());
    assert_eq!(IconState::Success.transient_duration_ms(), Some(2000));
    assert_eq!(IconState::Idle.transient_duration_ms(), None);
}
