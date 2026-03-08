use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::bridge::SisterId;

/// Circuit breaker state machine: Closed → Open → HalfOpen → Closed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Normal operation — calls pass through
    Closed,
    /// Too many failures — all calls rejected immediately
    Open,
    /// Recovery probe — one call allowed through to test
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time window for counting failures
    pub failure_window: Duration,
    /// How long to stay open before trying half-open
    pub recovery_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker for a single sister bridge
pub struct CircuitBreaker {
    sister_id: SisterId,
    config: CircuitBreakerConfig,
    state: Mutex<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: Mutex<Option<Instant>>,
    opened_at: Mutex<Option<Instant>>,
    total_rejections: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(sister_id: SisterId, config: CircuitBreakerConfig) -> Self {
        Self {
            sister_id,
            config,
            state: Mutex::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: Mutex::new(None),
            opened_at: Mutex::new(None),
            total_rejections: AtomicU64::new(0),
        }
    }

    pub fn with_defaults(sister_id: SisterId) -> Self {
        Self::new(sister_id, CircuitBreakerConfig::default())
    }

    /// Check if a call should be allowed through
    pub fn allow_call(&self) -> bool {
        let mut state = self.state.lock();

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                let opened_at = self.opened_at.lock();
                if let Some(opened) = *opened_at {
                    if opened.elapsed() >= self.config.recovery_timeout {
                        // Transition to half-open — allow one probe call
                        *state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                self.total_rejections.fetch_add(1, Ordering::Relaxed);
                false
            }
            CircuitState::HalfOpen => {
                // Already in half-open, only one probe allowed
                // Subsequent calls are rejected until probe resolves
                self.total_rejections.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    /// Record a successful call
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Relaxed);
        let mut state = self.state.lock();

        match *state {
            CircuitState::HalfOpen => {
                // Probe succeeded — close the circuit
                *state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::Relaxed);
                *self.opened_at.lock() = None;
                *self.last_failure_time.lock() = None;
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                // Only if we're outside the failure window
                let last_fail = self.last_failure_time.lock();
                if let Some(last) = *last_fail {
                    if last.elapsed() >= self.config.failure_window {
                        self.failure_count.store(0, Ordering::Relaxed);
                    }
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let now = Instant::now();
        let mut state = self.state.lock();

        match *state {
            CircuitState::Closed => {
                // Check if we're still in the failure window
                let mut last_fail = self.last_failure_time.lock();
                if let Some(last) = *last_fail {
                    if last.elapsed() >= self.config.failure_window {
                        // Window expired, reset counter
                        self.failure_count.store(0, Ordering::Relaxed);
                    }
                }
                *last_fail = Some(now);
                drop(last_fail);

                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.config.failure_threshold {
                    // Trip the circuit
                    *state = CircuitState::Open;
                    *self.opened_at.lock() = Some(now);
                    tracing::warn!(
                        sister = %self.sister_id.name(),
                        failures = failures,
                        "Circuit breaker OPENED for {}",
                        self.sister_id.name()
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Probe failed — re-open the circuit
                *state = CircuitState::Open;
                *self.opened_at.lock() = Some(now);
                tracing::warn!(
                    sister = %self.sister_id.name(),
                    "Circuit breaker re-OPENED for {} (probe failed)",
                    self.sister_id.name()
                );
            }
            CircuitState::Open => {
                // Already open, update opened_at for fresh timeout
            }
        }
    }

    pub fn state(&self) -> CircuitState {
        *self.state.lock()
    }

    pub fn sister_id(&self) -> SisterId {
        self.sister_id
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::Relaxed)
    }

    pub fn total_rejections(&self) -> u64 {
        self.total_rejections.load(Ordering::Relaxed)
    }

    /// Force the circuit into a specific state (for testing)
    pub fn force_state(&self, new_state: CircuitState) {
        let mut state = self.state.lock();
        *state = new_state;
        if new_state == CircuitState::Open {
            *self.opened_at.lock() = Some(Instant::now());
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut state = self.state.lock();
        *state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        *self.opened_at.lock() = None;
        *self.last_failure_time.lock() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
