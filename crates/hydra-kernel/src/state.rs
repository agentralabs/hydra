//! Kernel state types — the instantaneous snapshot of Hydra's being.
//!
//! Every tick of the alive loop captures a `HydraState` snapshot.
//! This is NOT mutable global state — it is a value type passed through
//! the equation `dΨ/dt = L̂Ψ + ÂΨ + ĜΨ + ŜΨ − Γ̂Ψ`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The complete state of Hydra at a single instant.
/// This is the Ψ in the equation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraState {
    /// Position on the cognitive manifold.
    pub manifold_position: ManifoldPosition,

    /// Summary of the trust field across all tiers.
    pub trust_field: TrustFieldSummary,

    /// Current state of signal routing.
    pub signal_state: SignalState,

    /// Category-theoretic consistency state.
    pub category_state: CategoryState,

    /// Growth tracking state.
    pub growth_state: GrowthState,

    /// When this state was captured.
    pub captured_at: DateTime<Utc>,

    /// How many equation steps have been computed since boot.
    pub step_count: u64,

    /// The current Lyapunov stability value V(Psi).
    pub lyapunov_value: f64,
}

impl HydraState {
    /// Create an initial state at boot time.
    pub fn initial() -> Self {
        Self {
            manifold_position: ManifoldPosition::origin(),
            trust_field: TrustFieldSummary::default(),
            signal_state: SignalState::default(),
            category_state: CategoryState::default(),
            growth_state: GrowthState::default(),
            captured_at: Utc::now(),
            step_count: 0,
            lyapunov_value: 1.0,
        }
    }

    /// Returns true if the state is stable (Lyapunov positive).
    pub fn is_stable(&self) -> bool {
        self.lyapunov_value > crate::constants::LYAPUNOV_ALERT_THRESHOLD
    }

    /// Returns true if the state is critical (Lyapunov below critical threshold).
    pub fn is_critical(&self) -> bool {
        self.lyapunov_value < crate::constants::LYAPUNOV_CRITICAL_THRESHOLD
    }
}

/// Position on the cognitive manifold.
/// Represents where Hydra is in its abstract state space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldPosition {
    /// Coordinates in the state space (dimensionality varies).
    pub coordinates: Vec<f64>,

    /// The Laplace-Beltrami estimate at this position.
    pub laplace_beltrami: f64,
}

impl ManifoldPosition {
    /// The origin of the manifold (boot position).
    pub fn origin() -> Self {
        Self {
            coordinates: vec![0.0; 8],
            laplace_beltrami: 0.0,
        }
    }

    /// Compute the Laplace-Beltrami estimate from current coordinates.
    pub fn laplace_beltrami_estimate(&self) -> f64 {
        // Simplified: sum of squared coordinates as a proxy
        // for the Laplace-Beltrami operator on the manifold.
        let sum_sq: f64 = self.coordinates.iter().map(|x| x * x).sum();
        -sum_sq / (self.coordinates.len() as f64).max(1.0)
    }
}

/// Summary of trust relationships across all tiers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustFieldSummary {
    /// Average trust level (0.0 = no trust, 1.0 = full trust).
    pub average_trust: f64,

    /// Number of entities at each trust tier.
    pub tier_counts: Vec<usize>,

    /// Whether an adversarial condition has been detected.
    pub adversarial_detected: bool,
}

impl TrustFieldSummary {
    /// Estimate the adversarial pressure on the trust field.
    pub fn adversarial_estimate(&self) -> f64 {
        if self.adversarial_detected {
            -0.5
        } else {
            self.average_trust
        }
    }
}

/// State of signal routing through the Animus bus.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalState {
    /// Number of signals currently in flight.
    pub in_flight: usize,

    /// Number of signals dropped since last report.
    pub dropped: usize,

    /// Number of orphan signals detected since last report.
    pub orphans_detected: usize,

    /// Queue utilization as a fraction (0.0 = empty, 1.0 = full).
    pub queue_utilization: f64,
}

/// Category-theoretic consistency state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryState {
    /// Whether all functors are composing correctly.
    pub functors_consistent: bool,

    /// Number of natural transformation violations detected.
    pub nat_violations: usize,
}

impl CategoryState {
    /// Returns true if the category is healthy.
    pub fn is_healthy(&self) -> bool {
        self.functors_consistent && self.nat_violations == 0
    }
}

/// Growth tracking state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrowthState {
    /// Number of new capabilities acquired since boot.
    pub capabilities_acquired: usize,

    /// Number of beliefs revised since boot.
    pub beliefs_revised: usize,

    /// Growth rate estimate (capabilities per hour).
    pub growth_rate: f64,
}

/// Boot phases — the kernel progresses through these sequentially.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootPhase {
    /// Verifying the constitution is intact.
    ConstitutionVerify,
    /// Initializing the Animus bus.
    AnimusInit,
    /// Resuming memory from persistent storage.
    MemoryResume,
    /// Rehydrating belief state.
    BeliefRehydrate,
    /// Reconnecting to the fleet.
    FleetReconnect,
    /// Staging predictions for ambient processing.
    PredictionStage,
    /// TUI ready signal.
    TuiReady,
}

impl std::fmt::Display for BootPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstitutionVerify => write!(f, "constitution-verify"),
            Self::AnimusInit => write!(f, "animus-init"),
            Self::MemoryResume => write!(f, "memory-resume"),
            Self::BeliefRehydrate => write!(f, "belief-rehydrate"),
            Self::FleetReconnect => write!(f, "fleet-reconnect"),
            Self::PredictionStage => write!(f, "prediction-stage"),
            Self::TuiReady => write!(f, "tui-ready"),
        }
    }
}

/// The current phase of the kernel's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelPhase {
    /// Kernel is booting.
    Booting {
        /// Which boot phase is currently active.
        current: BootPhase,
    },
    /// Kernel is alive and running all three loops.
    Alive,
    /// Kernel is in degraded mode (some invariant failed).
    Degraded {
        /// Description of why the kernel is degraded.
        reason: String,
    },
    /// Kernel is shutting down.
    ShuttingDown,
    /// Kernel has stopped.
    Stopped,
}

impl KernelPhase {
    /// Returns true if the kernel is in a state that accepts commands.
    pub fn accepts_commands(&self) -> bool {
        matches!(self, Self::Alive | Self::Degraded { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_stable() {
        let state = HydraState::initial();
        assert!(state.is_stable());
        assert!(!state.is_critical());
        assert_eq!(state.step_count, 0);
    }

    #[test]
    fn critical_state_detected() {
        let mut state = HydraState::initial();
        state.lyapunov_value = -1.0;
        assert!(state.is_critical());
        assert!(!state.is_stable());
    }

    #[test]
    fn manifold_origin_is_zero() {
        let pos = ManifoldPosition::origin();
        assert_eq!(pos.laplace_beltrami_estimate(), 0.0);
    }

    #[test]
    fn laplace_beltrami_nonzero_for_offset() {
        let pos = ManifoldPosition {
            coordinates: vec![1.0, 1.0, 1.0, 1.0],
            laplace_beltrami: 0.0,
        };
        let lb = pos.laplace_beltrami_estimate();
        assert!(lb < 0.0);
    }

    #[test]
    fn trust_field_adversarial_estimate() {
        let mut tf = TrustFieldSummary::default();
        tf.average_trust = 0.8;
        assert_eq!(tf.adversarial_estimate(), 0.8);

        tf.adversarial_detected = true;
        assert_eq!(tf.adversarial_estimate(), -0.5);
    }

    #[test]
    fn category_state_health() {
        let mut cs = CategoryState::default();
        assert!(!cs.is_healthy()); // functors_consistent is false by default
        cs.functors_consistent = true;
        assert!(cs.is_healthy());
        cs.nat_violations = 1;
        assert!(!cs.is_healthy());
    }

    #[test]
    fn boot_phase_display() {
        assert_eq!(
            BootPhase::ConstitutionVerify.to_string(),
            "constitution-verify"
        );
        assert_eq!(BootPhase::TuiReady.to_string(), "tui-ready");
    }

    #[test]
    fn kernel_phase_accepts_commands() {
        assert!(KernelPhase::Alive.accepts_commands());
        assert!(
            KernelPhase::Degraded {
                reason: "test".to_string()
            }
            .accepts_commands()
        );
        assert!(!KernelPhase::Stopped.accepts_commands());
        assert!(!KernelPhase::ShuttingDown.accepts_commands());
    }
}
