//! Agent Swarm — spawn, manage, and coordinate up to 100+ agents.
//!
//! Combines P7 (persistence) and P8 (remote control) to distribute
//! work across local and remote agents.

pub mod agent;
pub mod spawner;
pub mod distributor;
pub mod aggregator;
pub mod monitor;
#[cfg(test)]
mod swarm_tests;

pub use agent::{
    AgentConfig, AgentHost, AgentId, AgentInstance, AgentPermissions,
    AgentRole, AgentStatus, AgentTask, Assignment, HealthStatus, TaskResult,
};
pub use spawner::SwarmSpawner;
pub use distributor::TaskDistributor;
pub use aggregator::{AggregatedReport, ResultAggregator};
pub use monitor::SwarmMonitor;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Central swarm manager — spawns, tracks, and coordinates agents.
pub struct SwarmManager {
    agents: Arc<RwLock<HashMap<String, AgentInstance>>>,
    spawner: SwarmSpawner,
    distributor: TaskDistributor,
    aggregator: ResultAggregator,
    monitor: SwarmMonitor,
    max_agents: usize,
}

impl SwarmManager {
    pub fn new(max_agents: usize) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            spawner: SwarmSpawner::new(),
            distributor: TaskDistributor::new(),
            aggregator: ResultAggregator::new(),
            monitor: SwarmMonitor::new(),
            max_agents,
        }
    }

    /// Spawn N local agents with a given role.
    pub async fn spawn_local(
        &self,
        count: usize,
        role: AgentRole,
        skills: Vec<String>,
    ) -> Vec<Result<AgentId, String>> {
        let mut results = Vec::new();
        let current = { self.agents.read().len() };
        let can_spawn = (self.max_agents - current).min(count);

        for i in 0..can_spawn {
            let config = AgentConfig {
                name: format!("agent-{}", i + current + 1),
                role: role.clone(),
                host: AgentHost::Local,
                skills: skills.clone(),
                permissions: AgentPermissions::default(),
                goal: None,
            };
            match self.spawner.spawn_local(&config).await {
                Ok(agent) => {
                    let id = agent.id.clone();
                    self.agents.write().insert(id.clone(), agent);
                    results.push(Ok(id));
                }
                Err(e) => results.push(Err(e)),
            }
        }

        if can_spawn < count {
            for _ in can_spawn..count {
                results.push(Err(format!(
                    "Max agents ({}) reached", self.max_agents
                )));
            }
        }

        results
    }

    /// Spawn agents on a remote host.
    pub async fn spawn_remote(
        &self,
        host: &str,
        user: &str,
        count: usize,
        role: AgentRole,
        skills: Vec<String>,
    ) -> Vec<Result<AgentId, String>> {
        let mut results = Vec::new();
        let current = { self.agents.read().len() };
        let can_spawn = (self.max_agents - current).min(count);

        for i in 0..can_spawn {
            let config = AgentConfig {
                name: format!("remote-agent-{}", i + current + 1),
                role: role.clone(),
                host: AgentHost::Remote {
                    host: host.to_string(),
                    user: user.to_string(),
                },
                skills: skills.clone(),
                permissions: AgentPermissions::default(),
                goal: None,
            };
            match self.spawner.spawn_remote(host, user, &config).await {
                Ok(agent) => {
                    let id = agent.id.clone();
                    self.agents.write().insert(id.clone(), agent);
                    results.push(Ok(id));
                }
                Err(e) => results.push(Err(e)),
            }
        }

        results
    }

    /// Distribute a task across all idle agents.
    pub fn assign_task(&self, goal: &str) -> Vec<Assignment> {
        let agents = self.agents.read();
        let idle_count = agents.values()
            .filter(|a| a.status == AgentStatus::Idle)
            .count();
        let tasks = self.distributor.decompose(goal, idle_count);
        let idle_agents: Vec<AgentInstance> = agents.values()
            .filter(|a| a.status == AgentStatus::Idle)
            .cloned()
            .collect();
        drop(agents);

        let assignments = self.distributor.assign(&tasks, &idle_agents);

        // Apply assignments
        let mut agents = self.agents.write();
        for assignment in &assignments {
            if let Some(agent) = agents.get_mut(&assignment.agent_id) {
                agent.assign_task(assignment.task.clone());
            }
        }

        assignments
    }

    /// Collect results from completed agents.
    pub fn collect_results(&self) -> AggregatedReport {
        let agents = self.agents.read();
        let results: Vec<TaskResult> = agents.values()
            .flat_map(|a| a.results.iter().cloned())
            .collect();
        self.aggregator.aggregate(&results)
    }

    /// Run health check on all agents.
    pub async fn health_check(&self) -> Vec<HealthStatus> {
        let agents = self.agents.read().clone();
        self.monitor.health_check(&agents).await
    }

    /// Terminate a specific agent.
    pub async fn kill_agent(&self, id: &str) -> Result<(), String> {
        // Extract agent, drop lock before await
        let mut agent = {
            let agents = self.agents.read();
            agents.get(id)
                .ok_or_else(|| format!("Agent {} not found", id))?
                .clone()
        };
        match &agent.host {
            AgentHost::Local => self.spawner.terminate_local(&mut agent).await,
            AgentHost::Remote { .. } => self.spawner.terminate_remote(&mut agent).await,
        }
        // Write back terminated status
        self.agents.write().insert(id.to_string(), agent);
        Ok(())
    }

    /// Terminate all agents.
    pub async fn kill_all(&self) {
        // Clone out, terminate, write back — avoids holding lock across await
        let mut snapshot: HashMap<String, AgentInstance> = self.agents.read().clone();
        self.monitor.terminate_all(&mut snapshot, &self.spawner).await;
        *self.agents.write() = snapshot;
    }

    /// Scale to N agents (spawn more or terminate excess).
    pub async fn scale_to(&self, target: usize) -> String {
        let current = self.agents.read().len();
        if target > current {
            let to_spawn = target - current;
            let results = self.spawn_local(
                to_spawn, AgentRole::Worker, vec![],
            ).await;
            let ok = results.iter().filter(|r| r.is_ok()).count();
            format!("Scaled up: spawned {} new agents ({} total)", ok, current + ok)
        } else if target < current {
            let to_kill = current - target;
            let ids: Vec<String> = self.agents.read()
                .keys().take(to_kill).cloned().collect();
            for id in &ids {
                if let Err(e) = self.kill_agent(id).await {
                    eprintln!("[hydra:swarm] kill_agent({}) FAILED: {}", id, e);
                }
            }
            format!("Scaled down: terminated {} agents ({} remaining)", ids.len(), target)
        } else {
            format!("Already at {} agents", current)
        }
    }

    /// Clone a handle to this manager (cheap — all state is Arc'd).
    pub fn clone_handle(&self) -> Self {
        Self {
            agents: self.agents.clone(),
            spawner: SwarmSpawner::new(),
            distributor: TaskDistributor::new(),
            aggregator: ResultAggregator::new(),
            monitor: SwarmMonitor::new(),
            max_agents: self.max_agents,
        }
    }

    /// Get agent count.
    pub fn agent_count(&self) -> usize {
        self.agents.read().len()
    }

    /// Get status summary of all agents.
    pub fn status_summary(&self) -> String {
        let agents = self.agents.read();
        if agents.is_empty() {
            return "No agents in swarm.\n\nUse /swarm spawn <count> to create agents.".into();
        }

        let idle = agents.values().filter(|a| a.status == AgentStatus::Idle).count();
        let working = agents.values().filter(|a| matches!(a.status, AgentStatus::Working(_))).count();
        let completed = agents.values().filter(|a| a.status == AgentStatus::Completed).count();
        let failed = agents.values().filter(|a| matches!(a.status, AgentStatus::Failed(_))).count();

        let mut out = format!(
            "Swarm Status ({} agents)\n\n  Idle: {}  Working: {}  Completed: {}  Failed: {}\n\n",
            agents.len(), idle, working, completed, failed,
        );
        for agent in agents.values() {
            out.push_str(&format!("  {}\n", agent.summary()));
        }
        out
    }

    /// Find agent by ID prefix (for slash commands).
    pub fn find_agent(&self, prefix: &str) -> Option<String> {
        self.agents.read()
            .keys()
            .find(|id| id.starts_with(prefix))
            .cloned()
    }
}

impl Default for SwarmManager {
    fn default() -> Self {
        Self::new(100)
    }
}
