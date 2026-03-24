//! Swarm coordinator — ties consensus, emergence, and health together.

use std::sync::{Arc, Mutex};

use crate::consensus::{detect_consensus, AgentAnswer, ConsensusSignal};
use crate::emergence::{EmergenceEntry, EmergenceStore};
use crate::errors::SwarmError;
use crate::health::SwarmHealth;
use hydra_fleet::{FleetAgentState, FleetRegistry};

/// The swarm coordinator.
pub struct Swarm {
    /// Shared fleet registry — can be shared with other subsystems.
    registry: Arc<Mutex<FleetRegistry>>,
    /// Emergence store for recording emergent behaviors.
    emergence: EmergenceStore,
    /// Current Lyapunov delta for stability tracking.
    lyapunov: f64,
}

impl Swarm {
    /// Create a new swarm wrapping a shared fleet registry.
    pub fn shared(registry: Arc<Mutex<FleetRegistry>>) -> Self {
        Self {
            registry,
            emergence: EmergenceStore::new(),
            lyapunov: 0.0,
        }
    }

    /// Create a new swarm with its own fleet registry.
    pub fn new(registry: FleetRegistry) -> Self {
        Self::shared(Arc::new(Mutex::new(registry)))
    }

    /// Return a clone of the shared registry handle.
    pub fn shared_registry(&self) -> Arc<Mutex<FleetRegistry>> {
        self.registry.clone()
    }

    /// Access fleet agent count.
    pub fn agent_count(&self) -> usize {
        self.registry.lock().map(|r| r.agent_count()).unwrap_or(0)
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
        let states: Vec<FleetAgentState> = self.registry.lock()
            .map(|r| r.agents().iter().map(|a| a.state).collect())
            .unwrap_or_default();
        SwarmHealth::compute(&states, self.lyapunov)
    }

    /// Update and return the Lyapunov delta based on current health.
    pub fn lyapunov_delta(&mut self) -> f64 {
        let health = self.health();
        self.lyapunov = health.lyapunov_delta;
        self.lyapunov
    }
}

impl std::fmt::Debug for Swarm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Swarm")
            .field("emergence_count", &self.emergence.count())
            .field("lyapunov", &self.lyapunov)
            .finish()
    }
}
