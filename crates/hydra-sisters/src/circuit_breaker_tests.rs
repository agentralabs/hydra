#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::bridge::SisterId;
    use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};

    #[test]
    fn test_starts_closed() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_call());
    }

    #[test]
    fn test_opens_after_threshold() {
        let cb = CircuitBreaker::new(
            SisterId::Memory,
            CircuitBreakerConfig {
                failure_threshold: 3,
                failure_window: Duration::from_secs(60),
                recovery_timeout: Duration::from_secs(30),
            },
        );

        // 3 failures should trip the circuit
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow_call());
    }

    #[test]
    fn test_rejects_when_open() {
        let cb = CircuitBreaker::with_defaults(SisterId::Vision);
        cb.force_state(CircuitState::Open);
        assert!(!cb.allow_call());
        assert!(cb.total_rejections() > 0);
    }

    #[test]
    fn test_half_open_after_recovery_timeout() {
        let cb = CircuitBreaker::new(
            SisterId::Memory,
            CircuitBreakerConfig {
                failure_threshold: 1,
                failure_window: Duration::from_secs(60),
                recovery_timeout: Duration::from_millis(1), // Tiny for test
            },
        );

        cb.record_failure(); // Opens circuit
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for recovery timeout
        std::thread::sleep(Duration::from_millis(5));

        // Should transition to half-open on next allow_call
        assert!(cb.allow_call());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_closes_on_successful_probe() {
        let cb = CircuitBreaker::with_defaults(SisterId::Codebase);
        cb.force_state(CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_reopens_on_failed_probe() {
        let cb = CircuitBreaker::with_defaults(SisterId::Identity);
        cb.force_state(CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_success_resets_failure_count() {
        let cb = CircuitBreaker::new(
            SisterId::Memory,
            CircuitBreakerConfig {
                failure_threshold: 5,
                failure_window: Duration::from_millis(1), // Tiny window
                recovery_timeout: Duration::from_secs(30),
            },
        );

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(5));

        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_reset() {
        let cb = CircuitBreaker::with_defaults(SisterId::Forge);
        cb.force_state(CircuitState::Open);
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_default_config() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.failure_window, Duration::from_secs(60));
        assert_eq!(config.recovery_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_with_defaults_constructor() {
        let cb = CircuitBreaker::with_defaults(SisterId::Aegis);
        assert_eq!(cb.sister_id(), SisterId::Aegis);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert_eq!(cb.success_count(), 0);
        assert_eq!(cb.total_rejections(), 0);
    }

    #[test]
    fn test_force_state_to_open() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::Open);
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_force_state_to_half_open() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::HalfOpen);
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_half_open_rejects_second_call() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::HalfOpen);
        // First call in HalfOpen is rejected (probe already sent)
        assert!(!cb.allow_call());
        assert!(cb.total_rejections() > 0);
    }

    #[test]
    fn test_success_count_increments() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.success_count(), 2);
    }

    #[test]
    fn test_failure_count_increments() {
        let cb = CircuitBreaker::new(
            SisterId::Memory,
            CircuitBreakerConfig {
                failure_threshold: 100,
                failure_window: Duration::from_secs(60),
                recovery_timeout: Duration::from_secs(30),
            },
        );
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 3);
    }

    #[test]
    fn test_reset_clears_all() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.record_failure();
        cb.record_failure();
        cb.force_state(CircuitState::Open);
        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
        assert!(cb.allow_call());
    }

    #[test]
    fn test_circuit_state_serialization() {
        let closed = CircuitState::Closed;
        let json = serde_json::to_string(&closed).unwrap();
        assert_eq!(json, "\"closed\"");

        let open = CircuitState::Open;
        let json = serde_json::to_string(&open).unwrap();
        assert_eq!(json, "\"open\"");

        let half = CircuitState::HalfOpen;
        let json = serde_json::to_string(&half).unwrap();
        assert_eq!(json, "\"half_open\"");
    }

    #[test]
    fn test_circuit_state_deserialization() {
        let s: CircuitState = serde_json::from_str("\"closed\"").unwrap();
        assert_eq!(s, CircuitState::Closed);
    }

    #[test]
    fn test_open_increments_total_rejections() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::Open);
        cb.allow_call();
        cb.allow_call();
        cb.allow_call();
        assert_eq!(cb.total_rejections(), 3);
    }

    #[test]
    fn test_failure_in_open_state_no_op() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::Open);
        let state_before = cb.state();
        cb.record_failure(); // Should not change state
        assert_eq!(cb.state(), state_before);
    }

    #[test]
    fn test_success_in_open_state_no_op() {
        let cb = CircuitBreaker::with_defaults(SisterId::Memory);
        cb.force_state(CircuitState::Open);
        cb.record_success(); // Should not change state from Open
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_failure_window_reset() {
        let cb = CircuitBreaker::new(
            SisterId::Memory,
            CircuitBreakerConfig {
                failure_threshold: 10,
                failure_window: Duration::from_millis(1),
                recovery_timeout: Duration::from_secs(30),
            },
        );
        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));
        cb.record_failure(); // Should reset counter because window expired
        assert_eq!(cb.failure_count(), 1);
    }
}
