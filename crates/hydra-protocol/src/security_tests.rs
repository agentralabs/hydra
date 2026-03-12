#[cfg(test)]
mod tests {
    use crate::security::*;
    use crate::types::ProtocolKind;
    use uuid::Uuid;

    // ── TransportSecurity tests ────────────────────────────

    #[test]
    fn test_transport_security_sister_is_local() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::Sister), TransportSecurity::Local);
    }

    #[test]
    fn test_transport_security_shell_is_local() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::ShellCommand), TransportSecurity::Local);
    }

    #[test]
    fn test_transport_security_mcp_is_local() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::McpTool), TransportSecurity::Local);
    }

    #[test]
    fn test_transport_security_rest_is_tls() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::RestApi), TransportSecurity::Tls13);
    }

    #[test]
    fn test_transport_security_browser_is_tls() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::BrowserAutomation), TransportSecurity::Tls13);
    }

    #[test]
    fn test_transport_security_llm_is_tls() {
        assert_eq!(TransportSecurity::required_for(ProtocolKind::LlmAgent), TransportSecurity::Tls13);
    }

    #[test]
    fn test_local_is_not_network() {
        assert!(!TransportSecurity::Local.is_network());
    }

    #[test]
    fn test_tls13_is_network() {
        assert!(TransportSecurity::Tls13.is_network());
    }

    #[test]
    fn test_mutual_tls_is_network() {
        assert!(TransportSecurity::MutualTls.is_network());
    }

    // ── verify_transport tests ─────────────────────────────

    #[test]
    fn test_verify_transport_local_protocol() {
        assert!(verify_transport(ProtocolKind::Sister, "anything").is_ok());
    }

    #[test]
    fn test_verify_transport_rest_https_ok() {
        assert!(verify_transport(ProtocolKind::RestApi, "https://api.example.com").is_ok());
    }

    #[test]
    fn test_verify_transport_rest_wss_ok() {
        assert!(verify_transport(ProtocolKind::RestApi, "wss://api.example.com").is_ok());
    }

    #[test]
    fn test_verify_transport_rest_http_fails() {
        assert!(verify_transport(ProtocolKind::RestApi, "http://api.example.com").is_err());
    }

    #[test]
    fn test_verify_transport_browser_http_fails() {
        assert!(verify_transport(ProtocolKind::BrowserAutomation, "http://bad.com").is_err());
    }

    #[test]
    fn test_verify_transport_rest_empty_endpoint_ok() {
        assert!(verify_transport(ProtocolKind::RestApi, "").is_ok());
    }

    // ── RateLimiter tests ──────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_first_call() {
        let limiter = RateLimiter::new();
        assert!(limiter.check(ProtocolKind::Sister).is_ok());
    }

    #[test]
    fn test_rate_limiter_call_count() {
        let limiter = RateLimiter::new();
        assert_eq!(limiter.call_count(ProtocolKind::Sister), 0);
        limiter.check(ProtocolKind::Sister).unwrap();
        assert_eq!(limiter.call_count(ProtocolKind::Sister), 1);
    }

    #[test]
    fn test_rate_limiter_exceeds_per_second() {
        let limiter = RateLimiter::new();
        // Browser has max 2/s
        limiter.check(ProtocolKind::BrowserAutomation).unwrap();
        limiter.check(ProtocolKind::BrowserAutomation).unwrap();
        assert!(limiter.check(ProtocolKind::BrowserAutomation).is_err());
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter = RateLimiter::default();
        assert!(limiter.check(ProtocolKind::Sister).is_ok());
    }

    // ── AuthVerifier tests ─────────────────────────────────

    #[test]
    fn test_auth_verifier_no_auth_required() {
        assert!(AuthVerifier::verify_before_execute(false, false, "test").is_ok());
    }

    #[test]
    fn test_auth_verifier_auth_required_valid() {
        assert!(AuthVerifier::verify_before_execute(true, true, "test").is_ok());
    }

    #[test]
    fn test_auth_verifier_auth_required_invalid() {
        assert!(AuthVerifier::verify_before_execute(true, false, "test").is_err());
    }

    // ── SignedHealthStatus tests ────────────────────────────

    #[test]
    fn test_signed_health_status_verify() {
        let status = SignedHealthStatus::new(
            Uuid::new_v4(),
            crate::health::HealthStatus::Healthy,
            0.99,
        );
        assert!(status.verify());
    }

    #[test]
    fn test_signed_health_status_tampered_fails() {
        let mut status = SignedHealthStatus::new(
            Uuid::new_v4(),
            crate::health::HealthStatus::Healthy,
            0.99,
        );
        status.uptime_ratio = 0.5; // tamper
        assert!(!status.verify());
    }

    #[test]
    fn test_signed_health_status_serialization() {
        let status = SignedHealthStatus::new(
            Uuid::new_v4(),
            crate::health::HealthStatus::Degraded,
            0.75,
        );
        let json = serde_json::to_string(&status).unwrap();
        let restored: SignedHealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.status, crate::health::HealthStatus::Degraded);
    }

    // ── ProtocolCallCounter tests ──────────────────────────

    #[test]
    fn test_call_counter_new() {
        let counter = ProtocolCallCounter::new();
        assert_eq!(counter.get(ProtocolKind::Sister), 0);
        assert_eq!(counter.get(ProtocolKind::RestApi), 0);
    }

    #[test]
    fn test_call_counter_increment() {
        let counter = ProtocolCallCounter::new();
        counter.increment(ProtocolKind::Sister);
        counter.increment(ProtocolKind::Sister);
        assert_eq!(counter.get(ProtocolKind::Sister), 2);
        assert_eq!(counter.get(ProtocolKind::RestApi), 0);
    }

    #[test]
    fn test_call_counter_default() {
        let counter = ProtocolCallCounter::default();
        assert_eq!(counter.get(ProtocolKind::LlmAgent), 0);
    }

    // ── TransportSecurity serialization ────────────────────

    #[test]
    fn test_transport_security_serialization() {
        let json = serde_json::to_string(&TransportSecurity::Tls13).unwrap();
        assert_eq!(json, "\"tls13\"");
        let json = serde_json::to_string(&TransportSecurity::Local).unwrap();
        assert_eq!(json, "\"local\"");
    }
}
