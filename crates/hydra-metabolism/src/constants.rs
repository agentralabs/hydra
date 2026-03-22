//! All constants for hydra-metabolism.
//! No magic numbers anywhere else in this crate.

/// Lyapunov value at or above which the system is optimal.
pub const LYAPUNOV_OPTIMAL: f64 = 0.3;

/// Lyapunov value at or above which the system is stable (but not optimal).
pub const LYAPUNOV_STABLE: f64 = 0.0;

/// Lyapunov value at or above which the system is in alert state.
pub const LYAPUNOV_ALERT: f64 = 0.0;

/// Lyapunov value below which the system is critical.
pub const LYAPUNOV_CRITICAL: f64 = -0.5;

/// Lyapunov value below which the system is in emergency.
pub const LYAPUNOV_EMERGENCY: f64 = -1.0;

/// Minimum allowed gamma-hat value. Below zero means the system is shrinking.
pub const GAMMA_HAT_FLOOR: f64 = 0.0;

/// How many Lyapunov values to retain for trend analysis.
pub const LYAPUNOV_HISTORY_WINDOW: usize = 100;

/// Maximum interventions allowed per hour before rate limiting.
pub const MAX_INTERVENTIONS_PER_HOUR: usize = 10;
