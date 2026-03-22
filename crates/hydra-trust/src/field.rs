//! Trust field — manages a collection of trust agents.

use crate::agent::TrustAgent;
use crate::constants::*;
use crate::errors::TrustError;
use crate::hamiltonian::{compute_hamiltonian, HamiltonianState};
use std::collections::HashMap;
use uuid::Uuid;

/// The trust field: a thermodynamic container for agents.
#[derive(Debug, Clone)]
pub struct TrustField {
    agents: HashMap<Uuid, TrustAgent>,
}

impl TrustField {
    /// Create an empty trust field.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Add an agent to the field.
    pub fn add_agent(&mut self, agent: TrustAgent) -> Result<(), TrustError> {
        if self.agents.len() >= MAX_AGENTS {
            return Err(TrustError::FieldAtCapacity {
                current: self.agents.len(),
                max: MAX_AGENTS,
            });
        }
        self.agents.insert(agent.id, agent);
        Ok(())
    }

    /// Get an agent by ID.
    pub fn get_agent(&self, id: &Uuid) -> Option<&TrustAgent> {
        self.agents.get(id)
    }

    /// Record a success for an agent.
    pub fn record_success(&mut self, agent_id: &Uuid) -> Result<(), TrustError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or_else(|| TrustError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?;
        if agent.is_quarantined() || agent.is_on_hold() {
            return Err(TrustError::AgentQuarantined {
                agent_id: agent_id.to_string(),
            });
        }
        agent.record_success();
        Ok(())
    }

    /// Record a failure for an agent.
    pub fn record_failure(
        &mut self,
        agent_id: &Uuid,
        reason: impl Into<String>,
    ) -> Result<(), TrustError> {
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or_else(|| TrustError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?;
        if agent.is_quarantined() || agent.is_on_hold() {
            return Err(TrustError::AgentQuarantined {
                agent_id: agent_id.to_string(),
            });
        }
        agent.record_failure(reason);
        Ok(())
    }

    /// Record a constitutional violation for an agent.
    /// This ALWAYS returns an Err (violations are always errors).
    pub fn record_constitutional_violation(
        &mut self,
        agent_id: &Uuid,
        reason: impl Into<String>,
    ) -> Result<(), TrustError> {
        let reason_str = reason.into();
        let agent = self
            .agents
            .get_mut(agent_id)
            .ok_or_else(|| TrustError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?;
        agent.record_constitutional_violation();
        Err(TrustError::ConstitutionalViolation {
            agent_id: agent_id.to_string(),
            reason: reason_str,
        })
    }

    /// Compute the Hamiltonian state of the field.
    pub fn hamiltonian(&self) -> HamiltonianState {
        let agents: Vec<TrustAgent> = self.agents.values().cloned().collect();
        compute_hamiltonian(&agents)
    }

    /// Return all active agents.
    pub fn active_agents(&self) -> Vec<&TrustAgent> {
        self.agents.values().filter(|a| a.is_active()).collect()
    }

    /// Return all quarantined agents.
    pub fn quarantined_agents(&self) -> Vec<&TrustAgent> {
        self.agents
            .values()
            .filter(|a| a.is_quarantined() || a.is_on_hold())
            .collect()
    }

    /// Return total number of agents.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for TrustField {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_retrieve_agent() {
        let mut field = TrustField::new();
        let agent = TrustAgent::new("test");
        let id = agent.id;
        field.add_agent(agent).unwrap();
        assert!(field.get_agent(&id).is_some());
    }

    #[test]
    fn violation_returns_err() {
        let mut field = TrustField::new();
        let agent = TrustAgent::new("test");
        let id = agent.id;
        field.add_agent(agent).unwrap();
        let result = field.record_constitutional_violation(&id, "bad");
        assert!(result.is_err());
    }
}
