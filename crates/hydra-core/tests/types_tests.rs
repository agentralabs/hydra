use hydra_core::error::HydraError;
use hydra_core::*;
use std::time::Duration;
use uuid::Uuid;

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

// ═══════════════════════════════════════════════════════════
// PROACTIVE UPDATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_proactive_update_all_6_variants() {
    let updates: Vec<ProactiveUpdate> = vec![
        ProactiveUpdate::Acknowledgment {
            message: "Got it!".into(),
        },
        ProactiveUpdate::Progress {
            percent: 50.0,
            message: "Working...".into(),
            deployment_id: None,
        },
        ProactiveUpdate::Event {
            title: "Sister connected".into(),
            detail: "memory module online".into(),
        },
        ProactiveUpdate::Decision {
            request: DecisionRequest {
                id: Uuid::new_v4(),
                question: "Proceed?".into(),
                options: vec![],
                timeout_seconds: Some(30),
                default: Some(0),
            },
        },
        ProactiveUpdate::Completion {
            summary: CompletionSummary {
                headline: "Done!".into(),
                actions: vec!["Created file".into()],
                changes: vec!["src/main.rs".into()],
                next_steps: vec!["Run tests".into()],
            },
        },
        ProactiveUpdate::Alert {
            level: AlertLevel::Warning,
            message: "Low token budget".into(),
            suggestion: Some("Consider conservation mode".into()),
        },
    ];
    assert_eq!(updates.len(), 6);
}

// ═══════════════════════════════════════════════════════════
// DECISION & COMPLETION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_decision_option_keyboard_shortcut() {
    let option = DecisionOption {
        label: "Yes".into(),
        description: Some("Approve the action".into()),
        risk_level: Some(RiskLevel::Low),
        keyboard_shortcut: Some("y".into()),
    };
    assert_eq!(option.keyboard_shortcut, Some("y".to_string()));
}

#[test]
fn test_completion_summary() {
    let summary = CompletionSummary {
        headline: "File created successfully".into(),
        actions: vec!["Created src/main.rs".into()],
        changes: vec!["src/main.rs (new)".into()],
        next_steps: vec!["Run cargo build".into(), "Run cargo test".into()],
    };
    assert_eq!(summary.next_steps.len(), 2);
}

// ═══════════════════════════════════════════════════════════
// COGNITIVE STATE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_cognitive_phases() {
    let phases = [
        CognitivePhase::Perceive,
        CognitivePhase::Think,
        CognitivePhase::Decide,
        CognitivePhase::Act,
        CognitivePhase::Learn,
    ];
    assert_eq!(phases.len(), 5);
}

#[test]
fn test_cognitive_state_has_goals_and_beliefs() {
    let state = CognitiveState {
        phase: CognitivePhase::Think,
        intent_id: Some(Uuid::new_v4()),
        context: serde_json::json!({"working_dir": "/tmp"}),
        goals: vec![Goal {
            goal_type: GoalType::Create,
            target: "file".into(),
            outcome: "created".into(),
            sub_goals: vec![],
        }],
        budget: TokenBudget::new(10_000),
        beliefs: vec![Belief {
            key: "language".into(),
            value: serde_json::json!("rust"),
            confidence: 0.95,
            source: "user_preference".into(),
        }],
        checkpoint: None,
    };
    assert_eq!(state.goals.len(), 1);
    assert_eq!(state.beliefs.len(), 1);
    assert_eq!(state.beliefs[0].confidence, 0.95);
}

// ═══════════════════════════════════════════════════════════
// CAPABILITY TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_capability_token_expired() {
    let token = CapabilityToken {
        id: Uuid::new_v4(),
        holder_id: Uuid::new_v4(),
        capabilities: vec![Capability::FileRead],
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        signature: "sig".to_string(),
    };
    assert!(token.is_expired());
    assert!(!token.has_capability(&Capability::FileRead));
}

#[test]
fn test_capability_token_valid() {
    let token = CapabilityToken {
        id: Uuid::new_v4(),
        holder_id: Uuid::new_v4(),
        capabilities: vec![Capability::FileRead, Capability::FileWrite],
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        signature: "sig".to_string(),
    };
    assert!(!token.is_expired());
    assert!(token.has_capability(&Capability::FileRead));
    assert!(token.has_capability(&Capability::FileWrite));
    assert!(!token.has_capability(&Capability::FileDelete));
}

// ═══════════════════════════════════════════════════════════
// IDENTITY & SESSION TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_trust_level_ordering() {
    assert!(TrustLevel::Untrusted < TrustLevel::Basic);
    assert!(TrustLevel::Basic < TrustLevel::Verified);
    assert!(TrustLevel::Verified < TrustLevel::Trusted);
    assert!(TrustLevel::Trusted < TrustLevel::Full);
}

#[test]
fn test_onboarding_default_state() {
    let state = OnboardingState::default();
    assert_eq!(state.current_step, OnboardingStep::Welcome);
    assert!(!state.completed);
    assert!(state.user_name.is_none());
    assert!(state.voice_enabled.is_none());
}

#[test]
fn test_config_default() {
    let config = HydraConfig::default();
    assert_eq!(config.core.token_budget, 100_000);
    assert!(config.security.enable_signing);
    assert!(config.execution.sandbox_mode);
    assert!(config.voice.is_none());
}

// ═══════════════════════════════════════════════════════════
// EVENT TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_event_type_names() {
    let event = HydraEvent::KernelStarted {
        version: "0.1.0".into(),
    };
    assert_eq!(event.event_type(), "kernel.started");

    let event = HydraEvent::DeploymentProgress {
        deployment_id: Uuid::new_v4(),
        step: "compiling".into(),
        progress: 50.0,
    };
    assert_eq!(event.event_type(), "deployment.progress");
}

// ═══════════════════════════════════════════════════════════
// ERROR TESTS — user_message, suggested_action, codes
// ═══════════════════════════════════════════════════════════

#[test]
fn test_error_user_messages_are_friendly() {
    let errors: Vec<HydraError> = vec![
        HydraError::CompilationError("parse failed".into()),
        HydraError::NoActionDetected,
        HydraError::NoProtocolsFound,
        HydraError::Timeout,
        HydraError::SisterNotFound("memory".into()),
        HydraError::TokenBudgetExceeded {
            needed: 1000,
            available: 500,
        },
        HydraError::SerializationError("bad json".into()),
    ];
    for err in &errors {
        let msg = err.user_message();
        assert!(
            !msg.contains("0x"),
            "Error message contains hex code: {msg}"
        );
        assert!(
            !msg.contains("panic"),
            "Error message contains 'panic': {msg}"
        );
        assert!(
            !msg.contains("stack"),
            "Error message contains 'stack': {msg}"
        );
    }
}

#[test]
fn test_error_codes_unique() {
    let errors: Vec<HydraError> = vec![
        HydraError::CompilationError("".into()),
        HydraError::NoActionDetected,
        HydraError::NoProtocolsFound,
        HydraError::AllProtocolsFailed("".into()),
        HydraError::DeploymentFailed("".into()),
        HydraError::ApprovalRequired,
        HydraError::Timeout,
        HydraError::SisterNotFound("".into()),
        HydraError::SisterUnreachable("".into()),
        HydraError::PermissionDenied("".into()),
        HydraError::ConfigError("".into()),
        HydraError::IoError("".into()),
        HydraError::ReceiptChainBroken(0),
        HydraError::TokenBudgetExceeded {
            needed: 0,
            available: 0,
        },
        HydraError::SessionNotFound("".into()),
        HydraError::SerializationError("".into()),
        HydraError::Internal("".into()),
    ];
    let codes: Vec<&str> = errors.iter().map(|e| e.error_code()).collect();
    let mut unique = codes.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(codes.len(), unique.len(), "Duplicate error codes found");
}

#[test]
fn test_error_suggested_actions() {
    let err = HydraError::SisterNotFound("memory".into());
    assert!(err.suggested_action().is_some());
    assert!(err.suggested_action().unwrap().contains("memory"));
}

#[test]
fn test_error_retryable() {
    assert!(HydraError::Timeout.is_retryable());
    assert!(HydraError::SisterUnreachable("x".into()).is_retryable());
    assert!(HydraError::IoError("x".into()).is_retryable());
    assert!(!HydraError::NoActionDetected.is_retryable());
    assert!(!HydraError::PermissionDenied("x".into()).is_retryable());
}

// ═══════════════════════════════════════════════════════════
// ERROR CONVERSION TESTS (5+ required)
// ═══════════════════════════════════════════════════════════

#[test]
fn test_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let hydra_err: HydraError = io_err.into();
    assert!(matches!(hydra_err, HydraError::IoError(_)));
    assert_eq!(hydra_err.error_code(), "E502");
}

#[test]
fn test_error_from_serde_json() {
    let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
    let hydra_err: HydraError = json_err.into();
    assert!(matches!(hydra_err, HydraError::SerializationError(_)));
    assert_eq!(hydra_err.error_code(), "E901");
}

#[test]
fn test_error_from_string() {
    let hydra_err: HydraError = "something went wrong".to_string().into();
    assert!(matches!(hydra_err, HydraError::Internal(_)));
    assert_eq!(hydra_err.error_code(), "E999");
}

#[test]
fn test_error_from_str() {
    let hydra_err: HydraError = "bad things".into();
    assert!(matches!(hydra_err, HydraError::Internal(_)));
}

#[test]
fn test_error_from_uuid() {
    let uuid_err = "not-a-uuid".parse::<Uuid>().unwrap_err();
    let hydra_err: HydraError = uuid_err.into();
    assert!(matches!(hydra_err, HydraError::SerializationError(_)));
}

#[test]
fn test_error_from_toml() {
    let toml_err = toml::from_str::<serde_json::Value>("{{bad").unwrap_err();
    let hydra_err: HydraError = toml_err.into();
    assert!(matches!(hydra_err, HydraError::ConfigError(_)));
    assert_eq!(hydra_err.error_code(), "E501");
}

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

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

fn make_compiled_intent(
    confidence: f64,
    steps: usize,
    action_types: Vec<ActionType>,
) -> CompiledIntent {
    let actions = action_types
        .into_iter()
        .map(|at| Action::new(at, "test_target"))
        .collect();
    CompiledIntent {
        id: Uuid::new_v4(),
        raw_text: "test intent".into(),
        source: IntentSource::Cli,
        goal: Goal {
            goal_type: GoalType::Create,
            target: "test".into(),
            outcome: "test outcome".into(),
            sub_goals: vec![],
        },
        entities: vec![],
        actions,
        constraints: vec![],
        success_criteria: vec![],
        confidence,
        estimated_steps: steps,
        tokens_used: 0,
        veritas_validation: VeritasValidation {
            validated: true,
            safety_score: 1.0,
            warnings: vec![],
        },
    }
}

fn make_receipt(seq: u64, prev_hash: Option<String>) -> Receipt {
    Receipt {
        id: Uuid::new_v4(),
        deployment_id: Uuid::new_v4(),
        receipt_type: ReceiptType::DeploymentComplete,
        timestamp: chrono::Utc::now(),
        content: serde_json::json!({}),
        content_hash: format!("hash{seq}"),
        signature: "sig".to_string(),
        previous_hash: prev_hash,
        sequence: seq,
    }
}
