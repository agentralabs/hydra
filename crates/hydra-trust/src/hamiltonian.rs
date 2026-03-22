//! Hamiltonian trust thermodynamics.
//!
//! Models the fleet as a thermodynamic system where trust is energy
//! and the Hamiltonian determines phase transitions.

use crate::agent::TrustAgent;
use crate::constants::*;
use serde::{Deserialize, Serialize};

/// The phase of the trust field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustPhase {
    /// Fleet is healthy — most agents trusted.
    Stable,
    /// Fleet has some concerns — moderate distrust.
    Elevated,
    /// Fleet is under stress — significant distrust.
    Critical,
    /// Fleet has collapsed — trust is gone.
    Collapsed,
}

impl std::fmt::Display for TrustPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable => write!(f, "Stable"),
            Self::Elevated => write!(f, "Elevated"),
            Self::Critical => write!(f, "Critical"),
            Self::Collapsed => write!(f, "Collapsed"),
        }
    }
}

/// The Hamiltonian state of the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HamiltonianState {
    /// Total energy of the fleet (sum of agent tier energies).
    pub total_energy: f64,
    /// Average trust score across all agents.
    pub average_trust: f64,
    /// Current phase of the fleet.
    pub phase: TrustPhase,
    /// Number of agents contributing to this state.
    pub agent_count: usize,
}

/// Compute the Hamiltonian state from a set of agents.
pub fn compute_hamiltonian(agents: &[TrustAgent]) -> HamiltonianState {
    if agents.is_empty() {
        return HamiltonianState {
            total_energy: 0.0,
            average_trust: 0.0,
            phase: TrustPhase::Stable,
            agent_count: 0,
        };
    }

    let total_energy: f64 = agents.iter().map(|a| a.tier().energy()).sum();
    let average_trust: f64 =
        agents.iter().map(|a| a.score.value()).sum::<f64>() / agents.len() as f64;

    let phase = if average_trust >= T_FLEET_TRUSTED {
        TrustPhase::Stable
    } else if average_trust >= T_FLEET_UNRELIABLE {
        TrustPhase::Elevated
    } else if average_trust > TRUST_SCORE_MIN {
        TrustPhase::Critical
    } else {
        TrustPhase::Collapsed
    };

    HamiltonianState {
        total_energy,
        average_trust,
        phase,
        agent_count: agents.len(),
    }
}

/// Apply a constitutional violation spike to the Hamiltonian.
/// Returns the new phase after the spike.
pub fn apply_violation_spike(state: &HamiltonianState) -> TrustPhase {
    let adjusted_trust = state.average_trust - CONSTITUTIONAL_VIOLATION_SPIKE;
    if adjusted_trust <= TRUST_SCORE_MIN {
        TrustPhase::Collapsed
    } else if adjusted_trust < T_FLEET_UNRELIABLE {
        TrustPhase::Critical
    } else if adjusted_trust < T_FLEET_TRUSTED {
        TrustPhase::Elevated
    } else {
        TrustPhase::Stable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_fleet_is_stable() {
        let state = compute_hamiltonian(&[]);
        assert_eq!(state.phase, TrustPhase::Stable);
        assert_eq!(state.agent_count, 0);
    }

    #[test]
    fn uniform_fleet_phase() {
        let agents: Vec<TrustAgent> = (0..5).map(|i| TrustAgent::new(format!("a{i}"))).collect();
        let state = compute_hamiltonian(&agents);
        // Default score is 0.5, which is Elevated
        assert_eq!(state.phase, TrustPhase::Elevated);
    }
}
