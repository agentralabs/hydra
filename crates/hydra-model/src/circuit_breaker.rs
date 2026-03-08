use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Normal operation, requests pass through
    Closed,
    /// Too many failures, requests blocked
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Per-model circuit breaker
/// Threshold: 5 failures in 60s → open
/// Recovery: 30s → half-open → probe → close
pub struct CircuitBreaker {
    state: Mutex<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure: Mutex<Option<Instant>>,
    last_state_change: Mutex<Instant>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure: Mutex::new(None),
            last_state_change: Mutex::new(Instant::now()),
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }

    /// Current state
    pub fn state(&self) -> CircuitState {
        let mut state = self.state.lock();
        // Check if open circuit should transition to half-open
        if *state == CircuitState::Open {
            let elapsed = self.last_state_change.lock().elapsed();
            if elapsed >= self.recovery_timeout {
                *state = CircuitState::HalfOpen;
                *self.last_state_change.lock() = Instant::now();
            }
        }
        *state
    }

    /// Is the circuit open (blocking requests)?
    pub fn is_open(&self) -> bool {
        self.state() == CircuitState::Open
    }

    /// Should we allow a probe request? (half-open state)
    pub fn should_probe(&self) -> bool {
        self.state() == CircuitState::HalfOpen
    }

    /// Record a failure
    pub fn track_failure(&self) {
        *self.last_failure.lock() = Some(Instant::now());
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

        // Check if we should open the circuit
        if count >= self.failure_threshold {
            let mut state = self.state.lock();
            if *state == CircuitState::Closed || *state == CircuitState::HalfOpen {
                *state = CircuitState::Open;
                *self.last_state_change.lock() = Instant::now();
                self.failure_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Record a success
    pub fn track_success(&self) {
        self.success_count.fetch_add(1, Ordering::SeqCst);

        let mut state = self.state.lock();
        if *state == CircuitState::HalfOpen {
            // Probe succeeded — close the circuit
            *state = CircuitState::Closed;
            *self.last_state_change.lock() = Instant::now();
            self.failure_count.store(0, Ordering::SeqCst);
        }
    }

    /// Reset the circuit breaker
    pub fn reset(&self) {
        *self.state.lock() = CircuitState::Closed;
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.last_state_change.lock() = Instant::now();
    }

    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }

    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::SeqCst)
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}
