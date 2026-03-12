use hydra_core::error::HydraError;
use hydra_core::*;
use uuid::Uuid;

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
