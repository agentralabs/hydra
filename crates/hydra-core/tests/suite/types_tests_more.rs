use hydra_core::*;
use std::time::Duration;
use uuid::Uuid;

use super::helpers::{make_compiled_intent, make_receipt};

// ═══════════════════════════════════════════════════════════
// SERIALIZATION ROUNDTRIP TESTS (15+ required)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_serde_intent() {
    let intent = Intent::new("test", IntentSource::Cli);
    let json = serde_json::to_string(&intent).unwrap();
    let de: Intent = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, intent.id);
    assert_eq!(de.text, intent.text);
}

#[test]
fn test_serde_compiled_intent() {
    let intent = make_compiled_intent(
        0.85,
        2,
        vec![ActionType::FileCreate, ActionType::FileModify],
    );
    let json = serde_json::to_string(&intent).unwrap();
    let de: CompiledIntent = serde_json::from_str(&json).unwrap();
    assert_eq!(de.confidence, intent.confidence);
    assert_eq!(de.estimated_steps, intent.estimated_steps);
    assert_eq!(de.tokens_used, intent.tokens_used);
}

#[test]
fn test_serde_action() {
    let action = Action::new(ActionType::Write, "src/main.rs");
    let json = serde_json::to_string(&action).unwrap();
    let de: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(de.action_type, ActionType::Write);
    assert_eq!(de.target, "src/main.rs");
}

#[test]
fn test_serde_action_result() {
    let result = ActionResult {
        success: true,
        output: serde_json::json!(42),
        side_effects: vec![],
        duration: Duration::from_millis(100),
    };
    let json = serde_json::to_string(&result).unwrap();
    let de: ActionResult = serde_json::from_str(&json).unwrap();
    assert!(de.success);
}

#[test]
fn test_serde_receipt() {
    let receipt = make_receipt(0, None);
    let json = serde_json::to_string(&receipt).unwrap();
    let de: Receipt = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, receipt.id);
    assert_eq!(de.sequence, receipt.sequence);
}

#[test]
fn test_serde_token_budget() {
    let budget = TokenBudget::new(50_000);
    let json = serde_json::to_string(&budget).unwrap();
    let de: TokenBudget = serde_json::from_str(&json).unwrap();
    assert_eq!(de.total, 50_000);
    assert_eq!(de.remaining, 50_000);
}

#[test]
fn test_serde_token_metrics() {
    let metrics = TokenMetrics {
        used: 500,
        cached_hits: 10,
        llm_calls: 5,
        efficiency: 0.9,
        tokens_saved_by_batching: 200,
    };
    let json = serde_json::to_string(&metrics).unwrap();
    let de: TokenMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(de.cached_hits, 10);
    assert_eq!(de.efficiency, 0.9);
}

#[test]
fn test_serde_risk_assessment() {
    let assessment = RiskAssessment {
        level: RiskLevel::High,
        factors: vec![RiskFactor {
            name: "destructive".into(),
            severity: RiskLevel::High,
            description: "deletes files".into(),
        }],
        mitigations: vec!["backup first".into()],
        requires_approval: true,
    };
    let json = serde_json::to_string(&assessment).unwrap();
    let de: RiskAssessment = serde_json::from_str(&json).unwrap();
    assert_eq!(de.level, RiskLevel::High);
    assert_eq!(de.factors.len(), 1);
    assert_eq!(de.mitigations.len(), 1);
}

#[test]
fn test_serde_cognitive_state() {
    let state = CognitiveState {
        phase: CognitivePhase::Perceive,
        intent_id: None,
        context: serde_json::json!({}),
        goals: vec![],
        budget: TokenBudget::new(1000),
        beliefs: vec![],
        checkpoint: None,
    };
    let json = serde_json::to_string(&state).unwrap();
    let de: CognitiveState = serde_json::from_str(&json).unwrap();
    assert_eq!(de.phase, CognitivePhase::Perceive);
}

#[test]
fn test_serde_icon_state() {
    for state in [
        IconState::Idle,
        IconState::Working,
        IconState::Success,
        IconState::Offline,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let de: IconState = serde_json::from_str(&json).unwrap();
        assert_eq!(de, state);
    }
}

#[test]
fn test_serde_proactive_update_acknowledgment() {
    let update = ProactiveUpdate::Acknowledgment {
        message: "On it!".into(),
    };
    let json = serde_json::to_string(&update).unwrap();
    let de: ProactiveUpdate = serde_json::from_str(&json).unwrap();
    match de {
        ProactiveUpdate::Acknowledgment { message } => assert_eq!(message, "On it!"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_serde_decision_request() {
    let req = DecisionRequest {
        id: Uuid::new_v4(),
        question: "Overwrite file?".into(),
        options: vec![DecisionOption {
            label: "Yes".into(),
            description: Some("Overwrite".into()),
            risk_level: Some(RiskLevel::Medium),
            keyboard_shortcut: Some("y".into()),
        }],
        timeout_seconds: Some(30),
        default: Some(0),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: DecisionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.options.len(), 1);
    assert_eq!(de.options[0].keyboard_shortcut, Some("y".to_string()));
}

#[test]
fn test_serde_completion_summary() {
    let summary = CompletionSummary {
        headline: "Done".into(),
        actions: vec!["created".into()],
        changes: vec!["file.rs".into()],
        next_steps: vec!["test".into()],
    };
    let json = serde_json::to_string(&summary).unwrap();
    let de: CompletionSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(de.headline, "Done");
}

#[test]
fn test_serde_hydra_event() {
    let event = HydraEvent::IntentCompiled {
        intent_id: Uuid::new_v4(),
        confidence: 0.92,
    };
    let json = serde_json::to_string(&event).unwrap();
    let de: HydraEvent = serde_json::from_str(&json).unwrap();
    match de {
        HydraEvent::IntentCompiled { confidence, .. } => {
            assert!((confidence - 0.92).abs() < f64::EPSILON)
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_serde_capability_token() {
    let token = CapabilityToken {
        id: Uuid::new_v4(),
        holder_id: Uuid::new_v4(),
        capabilities: vec![
            Capability::FileRead,
            Capability::SisterAccess("memory".into()),
        ],
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        signature: "sig".into(),
    };
    let json = serde_json::to_string(&token).unwrap();
    let de: CapabilityToken = serde_json::from_str(&json).unwrap();
    assert_eq!(de.capabilities.len(), 2);
}

#[test]
fn test_serde_hydra_config() {
    let config = HydraConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let de: HydraConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(de.core.token_budget, 100_000);
}

#[test]
fn test_serde_deployed_solution() {
    let solution = DeployedSolution {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        status: DeploymentStatus::Complete,
        protocol_used: ProtocolUsed {
            protocol_id: Uuid::new_v4(),
            protocol_name: "shell".into(),
            was_fallback: false,
        },
        artifacts: vec![],
        steps: vec![],
        receipts: vec![],
        changes: vec![],
        rollback_available: true,
        duration: Duration::from_secs(5),
    };
    let json = serde_json::to_string(&solution).unwrap();
    let de: DeployedSolution = serde_json::from_str(&json).unwrap();
    assert_eq!(de.status, DeploymentStatus::Complete);
    assert!(de.rollback_available);
}

#[test]
fn test_serde_hydra_error() {
    let err = HydraError::TokenBudgetExceeded {
        needed: 1000,
        available: 500,
    };
    let json = serde_json::to_string(&err).unwrap();
    let de: HydraError = serde_json::from_str(&json).unwrap();
    match de {
        HydraError::TokenBudgetExceeded { needed, available } => {
            assert_eq!(needed, 1000);
            assert_eq!(available, 500);
        }
        _ => panic!("wrong variant"),
    }
}
