//! Category 1: Unit Gap Fill — hydra-core edge cases and boundaries.

use hydra_core::*;

// === Error variant exhaustive tests ===

#[test]
fn test_every_error_variant_has_code() {
    let errors = vec![
        HydraError::CompilationError("test".into()),
        HydraError::NoActionDetected,
        HydraError::NoProtocolsFound,
        HydraError::AllProtocolsFailed("test".into()),
        HydraError::DeploymentFailed("test".into()),
        HydraError::ApprovalRequired,
        HydraError::Timeout,
        HydraError::SisterNotFound("test".into()),
        HydraError::SisterUnreachable("test".into()),
        HydraError::PermissionDenied("test".into()),
        HydraError::ConfigError("test".into()),
        HydraError::IoError("test".into()),
        HydraError::ReceiptChainBroken(1),
        HydraError::TokenBudgetExceeded {
            needed: 100,
            available: 50,
        },
        HydraError::SessionNotFound("test".into()),
        HydraError::SerializationError("test".into()),
        HydraError::Internal("test".into()),
    ];
    let codes: Vec<&str> = errors.iter().map(|e| e.error_code()).collect();
    // All codes must be unique
    let mut seen = std::collections::HashSet::new();
    for code in &codes {
        assert!(seen.insert(code), "duplicate error code: {}", code);
    }
    // Every error must have a user_message
    for err in &errors {
        assert!(!err.user_message().is_empty());
    }
}

#[test]
fn test_every_error_variant_display() {
    let errors = vec![
        HydraError::CompilationError("compilation failed".into()),
        HydraError::NoActionDetected,
        HydraError::Timeout,
        HydraError::TokenBudgetExceeded {
            needed: 1000,
            available: 500,
        },
    ];
    for err in &errors {
        let display = format!("{}", err);
        assert!(!display.is_empty(), "empty Display for {:?}", err);
    }
}

#[test]
fn test_retryable_only_expected_errors() {
    assert!(HydraError::Timeout.is_retryable());
    assert!(HydraError::SisterUnreachable("x".into()).is_retryable());
    assert!(HydraError::IoError("x".into()).is_retryable());
    assert!(!HydraError::PermissionDenied("x".into()).is_retryable());
    assert!(!HydraError::NoActionDetected.is_retryable());
    assert!(!HydraError::CompilationError("x".into()).is_retryable());
    assert!(!HydraError::ApprovalRequired.is_retryable());
    assert!(!HydraError::ConfigError("x".into()).is_retryable());
}

// === Builder pattern / boundary tests ===

#[test]
fn test_token_budget_boundary_zero() {
    let budget = TokenBudget::new(0);
    assert!(!budget.can_afford(1));
    assert_eq!(budget.used(), 0);
    assert!(budget.is_below_threshold()); // 0 remaining is below 25%
}

#[test]
fn test_token_budget_boundary_exact() {
    let mut budget = TokenBudget::new(100);
    budget.record_usage(100);
    assert!(!budget.can_afford(1));
    assert_eq!(budget.remaining, 0);
    assert_eq!(budget.utilization(), 1.0);
}

#[test]
fn test_token_budget_boundary_overflow() {
    let mut budget = TokenBudget::new(100);
    budget.record_usage(200); // more than total
    assert_eq!(budget.remaining, 0); // saturating
    assert!(!budget.can_afford(1));
}

#[test]
fn test_token_budget_threshold_boundary() {
    let mut budget = TokenBudget::new(100);
    budget.record_usage(74); // 26% remaining — NOT below threshold
    assert!(!budget.is_below_threshold());
    budget.record_usage(2); // 24% remaining — below threshold
    assert!(budget.is_below_threshold());
}

#[test]
fn test_intent_empty_text() {
    let intent = Intent::new("", IntentSource::Cli);
    assert!(intent.text.is_empty());
}

#[test]
fn test_intent_max_length() {
    let long_text = "a".repeat(100_000);
    let intent = Intent::new(&long_text, IntentSource::Api);
    assert_eq!(intent.text.len(), 100_000);
}

#[test]
fn test_action_all_types() {
    let types = vec![
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
    for action_type in types {
        let action = Action::new(action_type.clone(), "target");
        assert_eq!(action.action_type, action_type);
    }
}

#[test]
fn test_compiled_intent_destructive_actions() {
    let mut intent = CompiledIntent {
        id: uuid::Uuid::new_v4(),
        raw_text: "delete everything".into(),
        source: IntentSource::Cli,
        goal: Goal {
            goal_type: GoalType::Delete,
            target: "files".into(),
            outcome: "deleted".into(),
            sub_goals: vec![],
        },
        entities: vec![],
        actions: vec![Action::new(ActionType::FileDelete, "/tmp/test")],
        constraints: vec![],
        success_criteria: vec![],
        confidence: 0.9,
        estimated_steps: 1,
        tokens_used: 100,
        veritas_validation: VeritasValidation {
            validated: true,
            safety_score: 0.9,
            warnings: vec![],
        },
    };
    assert!(intent.has_destructive_actions());

    intent.actions = vec![Action::new(ActionType::Read, "file.txt")];
    assert!(!intent.has_destructive_actions());
}

// === Receipt chain validation ===

#[test]
fn test_receipt_chain_broken_middle() {
    let r1 = Receipt {
        id: uuid::Uuid::new_v4(),
        deployment_id: uuid::Uuid::new_v4(),
        receipt_type: ReceiptType::IntentCompiled,
        timestamp: chrono::Utc::now(),
        content: serde_json::json!({"step": 1}),
        content_hash: "hash1".into(),
        signature: String::new(),
        previous_hash: None,
        sequence: 0,
    };
    let r2 = Receipt {
        id: uuid::Uuid::new_v4(),
        deployment_id: uuid::Uuid::new_v4(),
        receipt_type: ReceiptType::ExecutionStarted,
        timestamp: chrono::Utc::now(),
        content: serde_json::json!({"step": 2}),
        content_hash: "hash2".into(),
        signature: String::new(),
        previous_hash: Some("wrong_hash".into()), // broken chain
        sequence: 1,
    };
    assert!(r1.is_chain_valid(None));
    assert!(!r2.is_chain_valid(Some(&r1)));
}

// === Capability token edge cases ===

#[test]
fn test_capability_token_expired() {
    let token = CapabilityToken {
        id: uuid::Uuid::new_v4(),
        holder_id: uuid::Uuid::new_v4(),
        capabilities: vec![Capability::FileRead],
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        signature: String::new(),
    };
    assert!(token.is_expired());
    assert!(!token.has_capability(&Capability::FileRead)); // expired = no capabilities
}

#[test]
fn test_capability_token_wrong_capability() {
    let token = CapabilityToken {
        id: uuid::Uuid::new_v4(),
        holder_id: uuid::Uuid::new_v4(),
        capabilities: vec![Capability::FileRead],
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        signature: String::new(),
    };
    assert!(token.has_capability(&Capability::FileRead));
    assert!(!token.has_capability(&Capability::FileWrite));
    assert!(!token.has_capability(&Capability::ShellExecute));
}

// === Config defaults ===

#[test]
fn test_config_default_sensible() {
    let config = HydraConfig::default();
    assert!(config.core.token_budget > 0);
    assert!(config.execution.max_retries > 0);
    assert!(config.execution.timeout_seconds > 0);
    assert!(config.security.enable_signing);
}

// === Event type names ===

#[test]
fn test_all_event_types_unique() {
    let events = vec![
        HydraEvent::SessionStarted {
            session_id: uuid::Uuid::new_v4(),
        },
        HydraEvent::IntentReceived {
            intent_id: uuid::Uuid::new_v4(),
            text: "t".into(),
        },
        HydraEvent::KernelStarted {
            version: "0.1.0".into(),
        },
        HydraEvent::KernelShuttingDown { reason: "r".into() },
        HydraEvent::Error {
            source: "test".into(),
            message: "e".into(),
        },
    ];
    let mut types = std::collections::HashSet::new();
    for e in &events {
        assert!(
            types.insert(e.event_type()),
            "duplicate event type: {}",
            e.event_type()
        );
    }
}
