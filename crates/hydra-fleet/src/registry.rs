//! Fleet registry — manages the lifecycle of all agents in a fleet.

use crate::agent::{AgentSpecialization, FleetAgent, FleetAgentState};
use crate::assignment::{find_agent, AssignmentStrategy};
use crate::constants::FLEET_MAX_AGENTS;
use crate::errors::FleetError;
use crate::result::{AgentResult, ResultReceipt};
use crate::spawn::{check_spawn, SpawnRequest};
use crate::task::FleetTask;
use hydra_trust::agent::TrustAgent;
use hydra_trust::TrustField;
use uuid::Uuid;

/// The fleet registry — owns all agents and their trust field.
#[derive(Debug)]
pub struct FleetRegistry {
    /// All fleet agents, keyed by ID.
    agents: Vec<FleetAgent>,
    /// The trust field tracking all agents.
    pub trust: TrustField,
    /// Receipts issued for results.
    receipts: Vec<ResultReceipt>,
}

impl FleetRegistry {
    /// Create a new empty fleet registry.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            trust: TrustField::new(),
            receipts: Vec::new(),
        }
    }

    /// Spawn a new agent after constitutional and trust checks.
    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        specialization: AgentSpecialization,
        causal_root: impl Into<String>,
        requester_trust_score: f64,
        requester_tier: hydra_trust::TrustTier,
    ) -> Result<Uuid, FleetError> {
        if self.agents.len() >= FLEET_MAX_AGENTS {
            return Err(FleetError::FleetAtCapacity {
                current: self.agents.len(),
                max: FLEET_MAX_AGENTS,
            });
        }

        let name_str = name.into();
        let request = SpawnRequest {
            name: name_str.clone(),
            specialization: specialization.clone(),
            causal_root: causal_root.into(),
            requester_trust_score,
            requester_tier,
        };

        let check = check_spawn(&request)?;
        if !check.permitted {
            let reason = check
                .rejection_reason
                .unwrap_or_else(|| "spawn not permitted".to_string());
            return Err(FleetError::ConstitutionalRejection { reason });
        }

        let agent = FleetAgent::new(name_str.clone(), specialization);
        let agent_id = agent.id;

        // Create a corresponding TrustAgent
        let mut trust_agent = TrustAgent::new(name_str);
        // Overwrite the trust agent's ID to match the fleet agent's ID
        trust_agent.id = agent_id;
        self.trust
            .add_agent(trust_agent)
            .map_err(FleetError::Trust)?;

        self.agents.push(agent);

        eprintln!("[fleet] spawned agent {agent_id}");
        Ok(agent_id)
    }

    /// Assign a task to the best available agent using the given strategy.
    pub fn assign_task(
        &mut self,
        task: FleetTask,
        strategy: &AssignmentStrategy,
    ) -> Result<Uuid, FleetError> {
        let idx = find_agent(&self.agents, &task.task_type, strategy).ok_or_else(|| {
            FleetError::AgentNotFound {
                agent_id: "no suitable agent".to_string(),
            }
        })?;

        let agent = &mut self.agents[idx];
        let agent_id = agent.id;
        agent.assign_task(task)?;

        eprintln!("[fleet] assigned task to agent {agent_id}");
        Ok(agent_id)
    }

    /// Submit a result for an agent. Issues receipt BEFORE updating trust.
    pub fn submit_result(&mut self, result: AgentResult) -> Result<ResultReceipt, FleetError> {
        // Issue receipt FIRST
        let receipt = ResultReceipt::issue(&result);
        self.receipts.push(receipt.clone());

        eprintln!(
            "[fleet] receipted result {} for agent {}",
            receipt.result_id, receipt.agent_id
        );

        // Now update the agent and trust
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == result.agent_id)
            .ok_or_else(|| FleetError::AgentNotFound {
                agent_id: result.agent_id.to_string(),
            })?;

        match result.outcome {
            crate::result::ResultOutcome::Success
            | crate::result::ResultOutcome::PartialSuccess => {
                agent.complete_task();
                let _ = self.trust.record_success(&result.agent_id);
            }
            crate::result::ResultOutcome::ConstitutionalViolation => {
                agent.quarantine();
                agent.tasks_failed += 1;
                let _ = self.trust.record_constitutional_violation(
                    &result.agent_id,
                    "constitutional violation in result",
                );
            }
            _ => {
                agent.tasks_failed += 1;
                if !agent.task_queue.is_empty() {
                    agent.task_queue.remove(0);
                }
                if agent.task_queue.is_empty() {
                    agent.state = FleetAgentState::Idle;
                }
                let _ = self.trust.record_failure(&result.agent_id, "task failed");
            }
        }

        Ok(receipt)
    }

    /// Quarantine an agent by ID.
    pub fn quarantine(&mut self, agent_id: &Uuid) -> Result<(), FleetError> {
        let agent = self
            .agents
            .iter_mut()
            .find(|a| a.id == *agent_id)
            .ok_or_else(|| FleetError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?;

        agent.quarantine();

        // record_constitutional_violation always returns Err — ignore it
        let _ = self
            .trust
            .record_constitutional_violation(agent_id, "manual quarantine");

        eprintln!("[fleet] quarantined agent {agent_id}");
        Ok(())
    }

    /// Return the number of agents in the fleet.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Return a reference to all agents.
    pub fn agents(&self) -> &[FleetAgent] {
        &self.agents
    }

    /// Return all issued receipts.
    pub fn receipts(&self) -> &[ResultReceipt] {
        &self.receipts
    }

    /// Find an agent by ID.
    pub fn get_agent(&self, id: &Uuid) -> Option<&FleetAgent> {
        self.agents.iter().find(|a| a.id == *id)
    }
}

impl Default for FleetRegistry {
    fn default() -> Self {
        Self::new()
    }
}
