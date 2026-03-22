//! Swarm health monitoring.

use crate::constants::{
    SWARM_HEALTH_MIN_ACTIVE_FRACTION, SWARM_LYAPUNOV_BONUS, SWARM_LYAPUNOV_PENALTY,
};
use hydra_fleet::FleetAgentState;
use serde::{Deserialize, Serialize};

/// The health level of the swarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmHealthLevel {
    /// All systems nominal.
    Healthy,
    /// Some agents are degraded but swarm is functional.
    Degraded,
    /// Critical — too many agents inactive.
    Critical,
}

impl std::fmt::Display for SwarmHealthLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Critical => "Critical",
        };
        write!(f, "{label}")
    }
}

/// A snapshot of swarm health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmHealth {
    /// The health level.
    pub level: SwarmHealthLevel,
    /// Total agents in the swarm.
    pub total_agents: usize,
    /// Number of active (idle or working) agents.
    pub active_agents: usize,
    /// Number of quarantined agents.
    pub quarantined_agents: usize,
    /// Fraction of agents that are active.
    pub active_fraction: f64,
    /// Lyapunov stability delta.
    pub lyapunov_delta: f64,
}

impl SwarmHealth {
    /// Compute swarm health from a set of fleet agent states.
    pub fn compute(states: &[FleetAgentState], previous_lyapunov: f64) -> Self {
        let total_agents = states.len();
        let active_agents = states
            .iter()
            .filter(|s| matches!(s, FleetAgentState::Idle | FleetAgentState::Working))
            .count();
        let quarantined_agents = states
            .iter()
            .filter(|s| {
                matches!(
                    s,
                    FleetAgentState::Quarantined | FleetAgentState::ConstitutionalHold
                )
            })
            .count();

        let active_fraction = if total_agents > 0 {
            active_agents as f64 / total_agents as f64
        } else {
            0.0
        };

        let level = if active_fraction >= SWARM_HEALTH_MIN_ACTIVE_FRACTION {
            SwarmHealthLevel::Healthy
        } else if active_fraction >= SWARM_HEALTH_MIN_ACTIVE_FRACTION / 2.0 {
            SwarmHealthLevel::Degraded
        } else {
            SwarmHealthLevel::Critical
        };

        let lyapunov_delta = match level {
            SwarmHealthLevel::Healthy => previous_lyapunov + SWARM_LYAPUNOV_BONUS,
            SwarmHealthLevel::Degraded => previous_lyapunov - SWARM_LYAPUNOV_PENALTY,
            SwarmHealthLevel::Critical => previous_lyapunov - SWARM_LYAPUNOV_PENALTY * 2.0,
        };

        Self {
            level,
            total_agents,
            active_agents,
            quarantined_agents,
            active_fraction,
            lyapunov_delta,
        }
    }

    /// Return a one-line status description.
    pub fn status_line(&self) -> String {
        format!(
            "[swarm] {} — {}/{} active ({:.0}%), quarantined: {}, lyapunov: {:.4}",
            self.level,
            self.active_agents,
            self.total_agents,
            self.active_fraction * 100.0,
            self.quarantined_agents,
            self.lyapunov_delta,
        )
    }
}
