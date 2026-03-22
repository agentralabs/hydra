//! Error types for hydra-metabolism.

use thiserror::Error;

/// All errors that can occur during metabolism monitoring.
#[derive(Debug, Error)]
pub enum MetabolismError {
    /// Growth invariant violated: gamma-hat fell below the floor.
    #[error("growth invariant violated: gamma_hat={gamma_hat:.4}, floor={floor:.4}")]
    GrowthInvariantViolation {
        /// The actual gamma-hat value.
        gamma_hat: f64,
        /// The floor that was violated.
        floor: f64,
    },

    /// Lyapunov value is not finite (NaN or infinity).
    #[error("Lyapunov value is not finite: {value}")]
    NonFiniteLyapunov {
        /// The non-finite value.
        value: f64,
    },

    /// Intervention rate limit exceeded.
    #[error("intervention rate limit exceeded: {count} in the last hour (max {max})")]
    InterventionRateLimited {
        /// Number of interventions in the window.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },

    /// Internal computation error.
    #[error("internal error: {0}")]
    Internal(String),
}
