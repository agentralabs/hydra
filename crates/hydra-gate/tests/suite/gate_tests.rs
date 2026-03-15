use std::time::{Duration, Instant};

use hydra_core::types::{Action, ActionType, Capability, CapabilityToken, RiskLevel};
use hydra_gate::gate::{ExecutionGate, GateConfig, GateDecision};
use hydra_gate::kill_switch::KillSwitch;
use hydra_gate::risk::{ActionContext, RiskAssessor};
use hydra_gate::security_layers::{self, PerimeterConfig, ResourceLimits, SessionContext};

/// Gate with blocking enabled (warn_only = false) for tests that assert is_blocked()
fn enforcing_gate() -> ExecutionGate {
    ExecutionGate::new(GateConfig { warn_only: false, ..GateConfig::default() })
}

fn read_action() -> Action {
    Action::new(ActionType::Read, "src/main.rs")
}
fn write_action() -> Action {
    Action::new(ActionType::FileModify, "src/main.rs")
}
fn delete_action() -> Action {
    Action::new(ActionType::FileDelete, "src/old.rs")
}
fn shell_action() -> Action {
    Action::new(ActionType::ShellExecute, "rm -rf /tmp/test")
}
fn system_action() -> Action {
    Action::new(ActionType::System, "reboot")
}
fn network_action() -> Action {
    Action::new(ActionType::Network, "https://api.github.com/repos")
}
fn network_http_action() -> Action {
    Action::new(ActionType::Network, "http://insecure.com")
}
fn hydra_config_action() -> Action {
    Action::new(ActionType::FileModify, "/home/user/.hydra/config.toml")
}
fn unknown_action() -> Action {
    Action::new(ActionType::Composite, "unknown_complex_operation")
}
fn default_context() -> ActionContext {
    ActionContext::default()
}

// ═══════════════════════════════════════════════════════════
// RISK ASSESSOR TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_read_is_low_risk() {
    let assessor = RiskAssessor::new();
    let assessment = assessor.assess_risk_fast(&read_action(), &default_context());
    assert_eq!(assessment.level, RiskLevel::None);
    assert!(!assessment.requires_approval);
}

#[test]
fn test_delete_is_medium_risk() {
    let assessor = RiskAssessor::new();
    let assessment = assessor.assess_risk_fast(&delete_action(), &default_context());
    assert!(assessment.level >= RiskLevel::Low);
}

#[test]
fn test_shell_is_high_risk() {
    let assessor = RiskAssessor::new();
    let mut ctx = default_context();
    ctx.in_sandbox = false;
    let assessment = assessor.assess_risk_fast(&shell_action(), &ctx);
    assert!(assessment.level >= RiskLevel::Medium);
    assert!(assessment.requires_approval);
}

#[test]
fn test_system_is_high_risk() {
    let assessor = RiskAssessor::new();
    let assessment = assessor.assess_risk_fast(&system_action(), &default_context());
    assert!(assessment.level >= RiskLevel::Medium);
}

#[test]
fn test_self_modification_is_critical() {
    let assessor = RiskAssessor::new();
    let ctx = ActionContext {
        is_hydra_internal: true,
        ..Default::default()
    };
    let assessment = assessor.assess_risk_fast(&hydra_config_action(), &ctx);
    assert_eq!(assessment.level, RiskLevel::Critical);
}

// ═══════════════════════════════════════════════════════════
// GATE EVALUATION TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_auto_approve_low_risk() {
    let gate = ExecutionGate::default();
    let decision = gate
        .evaluate(&read_action(), &default_context(), None)
        .await;
    assert!(decision.is_approved());
    assert!(matches!(decision, GateDecision::AutoApprove { .. }));
}

#[tokio::test]
async fn test_require_approval_high_risk() {
    let gate = enforcing_gate();
    let mut ctx = default_context();
    ctx.in_sandbox = false;
    let decision = gate.evaluate(&shell_action(), &ctx, None).await;
    assert!(decision.needs_approval() || decision.is_blocked());
}

#[tokio::test]
async fn test_block_critical_risk() {
    let gate = enforcing_gate();
    let ctx = ActionContext {
        is_hydra_internal: true,
        ..Default::default()
    };
    let decision = gate.evaluate(&hydra_config_action(), &ctx, None).await;
    assert!(decision.is_blocked());
}

#[tokio::test]
async fn test_gate_generates_audit_log() {
    let gate = ExecutionGate::default();
    gate.evaluate(&read_action(), &default_context(), None)
        .await;
    gate.evaluate(&delete_action(), &default_context(), None)
        .await;
    let log = gate.audit_log();
    assert_eq!(log.len(), 2);
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 1 — PERIMETER
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_layer1_tls_blocks_http() {
    let gate = enforcing_gate();
    let decision = gate
        .evaluate(&network_http_action(), &default_context(), None)
        .await;
    assert!(decision.is_blocked());
}

#[tokio::test]
async fn test_layer1_tls_allows_https() {
    let gate = ExecutionGate::default();
    let decision = gate
        .evaluate(&network_action(), &default_context(), None)
        .await;
    assert!(!decision.is_blocked());
}

#[test]
fn test_layer1_rate_limiting() {
    let config = PerimeterConfig::new().with_rate_limit(5);
    let action = network_action();
    for _ in 0..5 {
        assert!(security_layers::check_perimeter_with_config(&action, &config).is_ok());
    }
    assert!(security_layers::check_perimeter_with_config(&action, &config).is_err());
}

#[test]
fn test_layer1_domain_allowlist() {
    let mut config = PerimeterConfig::new();
    config.allowed_domains.clear();
    config.allowed_domains.insert("api.github.com".into());
    let allowed = Action::new(ActionType::Network, "https://api.github.com/repos");
    assert!(security_layers::check_perimeter_with_config(&allowed, &config).is_ok());
    let blocked = Action::new(ActionType::Network, "https://evil.example.com/steal");
    assert!(security_layers::check_perimeter_with_config(&blocked, &config).is_err());
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 2 — AUTHENTICATION + SESSION
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_layer2_expired_token_blocked() {
    let gate = enforcing_gate();
    let expired = CapabilityToken {
        id: uuid::Uuid::new_v4(),
        holder_id: uuid::Uuid::new_v4(),
        capabilities: vec![Capability::FileRead],
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        signature: "sig".into(),
    };
    let decision = gate
        .evaluate(&read_action(), &default_context(), Some(&expired))
        .await;
    assert!(decision.is_blocked());
}

#[test]
fn test_layer2_session_management() {
    let valid = SessionContext {
        session_id: Some("abc-123".into()),
        ..Default::default()
    };
    assert!(security_layers::check_session(&valid).is_ok());
    let empty = SessionContext {
        session_id: Some("".into()),
        ..Default::default()
    };
    assert!(security_layers::check_session(&empty).is_err());
    let none = SessionContext::default();
    assert!(security_layers::check_session(&none).is_ok());
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 3 — AUTHORIZATION
// ═══════════════════════════════════════════════════════════

#[test]
fn test_layer3_capability_check() {
    let token = CapabilityToken {
        id: uuid::Uuid::new_v4(),
        holder_id: uuid::Uuid::new_v4(),
        capabilities: vec![Capability::FileRead], // Only read
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        signature: "sig".into(),
    };
    assert!(security_layers::check_authorization(&read_action(), Some(&token)).is_ok());
    assert!(security_layers::check_authorization(&delete_action(), Some(&token)).is_err());
}

#[test]
fn test_layer3_time_bounded_permissions() {
    let expired = CapabilityToken {
        id: uuid::Uuid::new_v4(),
        holder_id: uuid::Uuid::new_v4(),
        capabilities: vec![Capability::FileRead],
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        signature: "sig".into(),
    };
    // Expired token has no capabilities
    assert!(!expired.has_capability(&Capability::FileRead));
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 4 — EXECUTION CONTROL
// ═══════════════════════════════════════════════════════════

#[test]
fn test_layer4_resource_limits_configured() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.max_execution_time, Duration::from_secs(300));
    assert_eq!(limits.max_memory_mb, 1024);
    assert_eq!(limits.max_cpu_percent, 80);
}

// ═══════════════════════════════════════════════════════════
// SECURITY LAYER 5 — DATA PROTECTION
// ═══════════════════════════════════════════════════════════

#[test]
fn test_layer5_sanitize_secrets() {
    let input = "connecting to api_key=sk-abc123 server";
    let sanitized = security_layers::sanitize_for_output(input);
    assert!(
        !sanitized.contains("sk-abc123"),
        "Secret should be redacted: {sanitized}"
    );
    assert!(sanitized.contains("[REDACTED]"));
}

#[test]
fn test_layer5_data_isolation() {
    let session = SessionContext {
        project_id: Some("my-project".into()),
        ..Default::default()
    };
    let inside = Action::new(ActionType::FileModify, "/home/user/my-project/src/main.rs");
    assert!(security_layers::check_data_isolation(&inside, &session).is_ok());
    let outside = Action::new(
        ActionType::FileModify,
        "/home/user/other-project/secrets.rs",
    );
    assert!(security_layers::check_data_isolation(&outside, &session).is_err());
}
