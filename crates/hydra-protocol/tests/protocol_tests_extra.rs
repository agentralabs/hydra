use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use hydra_core::error::HydraError;
use hydra_protocol::health::{HealthStatus, HealthTracker};
use hydra_protocol::hunter::ProtocolHunter;
use hydra_protocol::registry::ProtocolRegistry;
use hydra_protocol::types::{Protocol, ProtocolEntry, ProtocolKind};

// ═══════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════

fn make_sister(name: &str, cap: &str) -> ProtocolEntry {
    ProtocolEntry::new(name, ProtocolKind::Sister).with_capabilities(vec![cap])
}

fn make_shell(name: &str, cap: &str) -> ProtocolEntry {
    ProtocolEntry::new(name, ProtocolKind::ShellCommand).with_capabilities(vec![cap])
}

fn make_rest(name: &str, cap: &str) -> ProtocolEntry {
    ProtocolEntry::new(name, ProtocolKind::RestApi).with_capabilities(vec![cap])
}

// ═══════════════════════════════════════════════════════════
// EDGE CASES (EC-PH-010)
// ═══════════════════════════════════════════════════════════

/// EC-PH-010: Sister not responding (health-based filtering)
#[tokio::test]
async fn test_ec_ph_010_sister_not_responding() {
    let reg = ProtocolRegistry::new();
    let proto = make_sister("unresponsive", "remember");
    let id = proto.id;
    reg.register(proto);
    // Also register a healthy alternative
    reg.register(make_shell("backup", "remember"));

    // Mark sister as unhealthy (3 consecutive failures)
    for _ in 0..3 {
        reg.mark_unhealthy(id);
    }

    let hunter = ProtocolHunter::new(reg);
    let start = std::time::Instant::now();
    let result = hunter.discover_with_timeout("remember").await;
    let elapsed = start.elapsed();

    // Should complete quickly (< 5s), not hang
    assert!(elapsed < Duration::from_secs(5));
    // Should skip unhealthy sister, use backup
    assert!(result.is_ok());
    let disc = result.unwrap();
    assert!(disc.primary.is_some());
    assert_eq!(disc.primary.unwrap().protocol.name, "backup");
}

// ═══════════════════════════════════════════════════════════
// ADDITIONAL TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_protocol_entry_is_usable() {
    let mut proto = make_rest("api", "action");
    assert!(proto.is_usable());

    proto.available = false;
    assert!(!proto.is_usable());

    proto.available = true;
    proto.auth_required = true;
    proto.auth_valid = false;
    assert!(!proto.is_usable());

    proto.auth_valid = true;
    assert!(proto.is_usable());
}

#[test]
fn test_mark_all_unhealthy() {
    let tracker = HealthTracker::new();
    let ids: Vec<_> = (0..5)
        .map(|_| {
            let id = Uuid::new_v4();
            tracker.mark_healthy(id);
            id
        })
        .collect();

    tracker.mark_all_unhealthy();
    for id in ids {
        assert_eq!(tracker.check_health(id), HealthStatus::Unhealthy);
    }
}

#[test]
fn test_registry_circular_dependency_detection() {
    let reg = ProtocolRegistry::new();
    let mut a = ProtocolEntry::new("A", ProtocolKind::Sister);
    let mut b = ProtocolEntry::new("B", ProtocolKind::Sister);
    let mut c = ProtocolEntry::new("C", ProtocolKind::Sister);
    let id_a = a.id;
    let id_b = b.id;
    let id_c = c.id;

    // A→B→C→A cycle
    a.depends_on.push(id_b);
    b.depends_on.push(id_c);
    c.depends_on.push(id_a);
    reg.register(a);
    reg.register(b);
    reg.register(c);

    assert!(reg.has_circular_dependency(id_a));
    assert!(reg.has_circular_dependency(id_b));
    assert!(reg.has_circular_dependency(id_c));
}

// ═══════════════════════════════════════════════════════════
// PROTOCOL TRAIT TESTS
// ═══════════════════════════════════════════════════════════

struct MockProtocol {
    proto_name: String,
    kind: ProtocolKind,
    available: bool,
}

#[async_trait]
impl Protocol for MockProtocol {
    fn name(&self) -> &str {
        &self.proto_name
    }
    fn protocol_type(&self) -> ProtocolKind {
        self.kind
    }
    fn is_available(&self) -> bool {
        self.available
    }
    async fn health(&self) -> HealthStatus {
        if self.available {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        }
    }
    async fn execute(
        &self,
        action: &str,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, HydraError> {
        Ok(serde_json::json!({"executed": action, "by": self.proto_name}))
    }
}

#[tokio::test]
async fn test_protocol_trait_execute() {
    let proto = MockProtocol {
        proto_name: "test-sister".into(),
        kind: ProtocolKind::Sister,
        available: true,
    };
    assert_eq!(proto.name(), "test-sister");
    assert_eq!(proto.protocol_type(), ProtocolKind::Sister);
    assert_eq!(proto.token_cost(), 100); // Sister default
    assert!(proto.is_available());

    let health = proto.health().await;
    assert_eq!(health, HealthStatus::Healthy);

    let result = proto
        .execute("deploy", serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(result["executed"], "deploy");
    assert_eq!(result["by"], "test-sister");
}

#[tokio::test]
async fn test_protocol_trait_unavailable() {
    let proto = MockProtocol {
        proto_name: "down".into(),
        kind: ProtocolKind::RestApi,
        available: false,
    };
    assert!(!proto.is_available());
    assert_eq!(proto.health().await, HealthStatus::Unhealthy);
    assert_eq!(proto.token_cost(), 500); // RestApi
}

// ═══════════════════════════════════════════════════════════
// AUTO-RECOVERY TESTS
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn test_auto_recovery_pings() {
    let tracker = Arc::new(HealthTracker::new());
    let id = Uuid::new_v4();

    // Mark unhealthy
    for _ in 0..3 {
        tracker.mark_unhealthy(id);
    }
    assert_eq!(tracker.check_health(id), HealthStatus::Unhealthy);
    assert_eq!(tracker.unhealthy_protocols().len(), 1);

    // Start auto-recovery that always succeeds
    let handle = tracker.start_auto_recovery(
        Duration::from_millis(50),
        Arc::new(|_| true), // always recover
    );

    // Wait for recovery
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();

    assert_eq!(tracker.check_health(id), HealthStatus::Healthy);
    assert_eq!(tracker.unhealthy_protocols().len(), 0);
}

#[tokio::test]
async fn test_auto_recovery_selective() {
    let tracker = Arc::new(HealthTracker::new());
    let good_id = Uuid::new_v4();
    let bad_id = Uuid::new_v4();

    for _ in 0..3 {
        tracker.mark_unhealthy(good_id);
        tracker.mark_unhealthy(bad_id);
    }

    // Recovery function only recovers good_id
    let captured_good = good_id;
    let handle = tracker.start_auto_recovery(
        Duration::from_millis(50),
        Arc::new(move |id| id == captured_good),
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();

    assert_eq!(tracker.check_health(good_id), HealthStatus::Healthy);
    assert_eq!(tracker.check_health(bad_id), HealthStatus::Unhealthy);
}

#[test]
fn test_no_circular_dependency() {
    let reg = ProtocolRegistry::new();
    let mut a = ProtocolEntry::new("A", ProtocolKind::Sister);
    let b = ProtocolEntry::new("B", ProtocolKind::Sister);
    let id_b = b.id;
    a.depends_on.push(id_b); // A→B, no cycle
    reg.register(a);
    reg.register(b);

    assert!(!reg.has_circular_dependency(id_b));
}

// ═══════════════════════════════════════════════════════════
// SECURITY TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_transport_security_requirements() {
    use hydra_protocol::TransportSecurity;
    // Network protocols require TLS
    assert!(TransportSecurity::required_for(ProtocolKind::RestApi).is_network());
    assert!(TransportSecurity::required_for(ProtocolKind::BrowserAutomation).is_network());
    assert!(TransportSecurity::required_for(ProtocolKind::LlmAgent).is_network());
    // Local protocols don't need TLS
    assert!(!TransportSecurity::required_for(ProtocolKind::Sister).is_network());
    assert!(!TransportSecurity::required_for(ProtocolKind::ShellCommand).is_network());
}

#[test]
fn test_transport_rejects_http() {
    use hydra_protocol::security::verify_transport;
    // HTTP should be rejected for REST
    let result = verify_transport(ProtocolKind::RestApi, "http://api.example.com");
    assert!(result.is_err());
    // HTTPS should be accepted
    let result = verify_transport(ProtocolKind::RestApi, "https://api.example.com");
    assert!(result.is_ok());
    // Local protocols accept anything
    let result = verify_transport(ProtocolKind::ShellCommand, "/usr/bin/test");
    assert!(result.is_ok());
}

#[test]
fn test_rate_limiter() {
    use hydra_protocol::RateLimiter;
    let limiter = RateLimiter::new();
    // Shell commands: max 10/second
    for _ in 0..10 {
        assert!(limiter.check(ProtocolKind::ShellCommand).is_ok());
    }
    // 11th should fail
    assert!(limiter.check(ProtocolKind::ShellCommand).is_err());
    assert_eq!(limiter.call_count(ProtocolKind::ShellCommand), 10);
}

#[test]
fn test_rate_limiter_different_types_independent() {
    use hydra_protocol::RateLimiter;
    let limiter = RateLimiter::new();
    // Exhaust shell limit
    for _ in 0..10 {
        limiter.check(ProtocolKind::ShellCommand).ok();
    }
    // Sister should still work (separate limit)
    assert!(limiter.check(ProtocolKind::Sister).is_ok());
}

#[test]
fn test_auth_verifier_before_execute() {
    use hydra_protocol::AuthVerifier;
    // Auth not required → pass
    assert!(AuthVerifier::verify_before_execute(false, false, "test").is_ok());
    // Auth required + valid → pass
    assert!(AuthVerifier::verify_before_execute(true, true, "test").is_ok());
    // Auth required + invalid → reject
    let result = AuthVerifier::verify_before_execute(true, false, "github-api");
    assert!(result.is_err());
    match result.unwrap_err() {
        HydraError::PermissionDenied(msg) => {
            assert!(msg.contains("github-api"));
            assert!(msg.contains("authentication"));
        }
        other => panic!("Expected PermissionDenied, got {:?}", other),
    }
}

#[test]
fn test_signed_health_status() {
    use hydra_protocol::SignedHealthStatus;
    let signed = SignedHealthStatus::new(Uuid::new_v4(), HealthStatus::Healthy, 0.99);
    // Verify hash matches content
    assert!(signed.verify());
    assert!(!signed.content_hash.is_empty());
}

#[test]
fn test_signed_health_tamper_detection() {
    use hydra_protocol::SignedHealthStatus;
    let mut signed = SignedHealthStatus::new(Uuid::new_v4(), HealthStatus::Healthy, 0.99);
    // Tamper with status
    signed.status = HealthStatus::Unhealthy;
    // Verification should fail
    assert!(!signed.verify());
}
