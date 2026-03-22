//! Swarm coordinator — ties consensus, emergence, and health together.

use crate::consensus::{detect_consensus, AgentAnswer, ConsensusSignal};
use crate::emergence::{EmergenceEntry, EmergenceStore};
use crate::errors::SwarmError;
use crate::health::SwarmHealth;
use hydra_fleet::{FleetAgentState, FleetRegistry};

/// The swarm coordinator.
#[derive(Debug)]
pub struct Swarm {
    /// The fleet registry this swarm monitors.
    registry: FleetRegistry,
    /// Emergence store for recording emergent behaviors.
    emergence: EmergenceStore,
    /// Current Lyapunov delta for stability tracking.
    lyapunov: f64,
}

impl Swarm {
    /// Create a new swarm wrapping a fleet registry.
    pub fn new(registry: FleetRegistry) -> Self {
        Self {
            registry,
            emergence: EmergenceStore::new(),
            lyapunov: 0.0,
        }
    }

    /// Return a mutable reference to the underlying fleet registry.
    pub fn registry_mut(&mut self) -> &mut FleetRegistry {
        &mut self.registry
    }

    /// Return a reference to the underlying fleet registry.
    pub fn registry(&self) -> &FleetRegistry {
        &self.registry
    }

    /// Evaluate consensus among a set of agent answers.
    pub fn evaluate_consensus(
        &self,
        answers: &[AgentAnswer],
    ) -> Result<ConsensusSignal, SwarmError> {
        if answers.len() < crate::constants::CONSENSUS_MIN_AGENTS {
            return Err(SwarmError::InsufficientAgents {
                needed: crate::constants::CONSENSUS_MIN_AGENTS,
                have: answers.len(),
            });
        }
        Ok(detect_consensus(answers))
    }

    /// Record an emergence event in the append-only store.
    pub fn record_emergence(&mut self, entry: EmergenceEntry) -> bool {
        self.emergence.append(entry)
    }

    /// Return the current emergence count.
    pub fn emergence_count(&self) -> usize {
        self.emergence.count()
    }

    /// Return a reference to the emergence store.
    pub fn emergence_store(&self) -> &EmergenceStore {
        &self.emergence
    }

    /// Compute the current swarm health.
    pub fn health(&self) -> SwarmHealth {
        let states: Vec<FleetAgentState> = self.registry.agents().iter().map(|a| a.state).collect();
        SwarmHealth::compute(&states, self.lyapunov)
    }

    /// Update and return the Lyapunov delta based on current health.
    pub fn lyapunov_delta(&mut self) -> f64 {
        let health = self.health();
        self.lyapunov = health.lyapunov_delta;
        self.lyapunov
    }
}
