//! Health monitor — detect failed agents, replace them, terminate all.

use std::collections::HashMap;
use super::agent::{AgentHost, AgentInstance, AgentStatus, HealthStatus};
use super::spawner::SwarmSpawner;

/// Monitors swarm health — pings agents, detects failures, replaces dead agents.
pub struct SwarmMonitor;

impl SwarmMonitor {
    pub fn new() -> Self {
        Self
    }

    /// Check all agents are alive.
    pub async fn health_check(
        &self,
        agents: &HashMap<String, AgentInstance>,
    ) -> Vec<HealthStatus> {
        let mut statuses = Vec::new();

        for (id, agent) in agents {
            let status = match &agent.host {
                AgentHost::Local => self.check_local(agent).await,
                AgentHost::Remote { host, user } => {
                    self.check_remote(agent, host, user).await
                }
            };
            statuses.push(HealthStatus {
                agent_id: id.clone(),
                alive: status.0,
                responsive: status.1,
                error: status.2,
            });
        }

        statuses
    }

    /// Check a local agent by PID.
    async fn check_local(&self, agent: &AgentInstance) -> (bool, bool, Option<String>) {
        if agent.status == AgentStatus::Terminated {
            return (false, false, Some("Terminated".into()));
        }

        if let Some(pid) = agent.pid {
            // Check if process is still alive
            #[cfg(unix)]
            {
                let alive = unsafe { libc::kill(pid as i32, 0) } == 0;
                if alive {
                    (true, true, None)
                } else {
                    (false, false, Some(format!("Process {} not found", pid)))
                }
            }
            #[cfg(not(unix))]
            {
                let _ = pid;
                (true, true, None)
            }
        } else {
            // No PID — simulation mode, consider alive
            (true, true, None)
        }
    }

    /// Check a remote agent via SSH.
    async fn check_remote(
        &self,
        agent: &AgentInstance,
        host: &str,
        user: &str,
    ) -> (bool, bool, Option<String>) {
        if agent.status == AgentStatus::Terminated {
            return (false, false, Some("Terminated".into()));
        }

        let check_cmd = format!(
            "pgrep -f 'hydra-cli.*{}' >/dev/null 2>&1 && echo ALIVE || echo DEAD",
            &agent.id[..8],
        );

        match crate::remote::ssh_execute(host, user, &check_cmd).await {
            Ok(output) => {
                let alive = output.stdout.trim() == "ALIVE";
                (alive, alive, if alive { None } else { Some("Process not found".into()) })
            }
            Err(e) => (false, false, Some(format!("SSH check failed: {}", e))),
        }
    }

    /// Replace a failed agent with a new one using the same config.
    pub async fn replace_failed(
        &self,
        failed: &AgentInstance,
        spawner: &SwarmSpawner,
    ) -> Result<AgentInstance, String> {
        let config = super::agent::AgentConfig {
            name: format!("{}-replacement", failed.name),
            role: failed.role.clone(),
            host: failed.host.clone(),
            skills: failed.skills.clone(),
            permissions: failed.permissions.clone(),
            goal: failed.task.as_ref().map(|t| t.description.clone()),
        };

        eprintln!("[swarm:monitor] Replacing failed agent {} ({})",
            failed.name, &failed.id[..8]);

        match &failed.host {
            AgentHost::Local => spawner.spawn_local(&config).await,
            AgentHost::Remote { host, user } => {
                spawner.spawn_remote(host, user, &config).await
            }
        }
    }

    /// Terminate all agents in the swarm.
    pub async fn terminate_all(
        &self,
        agents: &mut HashMap<String, AgentInstance>,
        spawner: &SwarmSpawner,
    ) {
        for (_, agent) in agents.iter_mut() {
            if agent.status == AgentStatus::Terminated {
                continue;
            }
            match &agent.host {
                AgentHost::Local => spawner.terminate_local(agent).await,
                AgentHost::Remote { .. } => spawner.terminate_remote(agent).await,
            }
        }
        eprintln!("[swarm:monitor] All {} agents terminated", agents.len());
    }
}

impl Default for SwarmMonitor {
    fn default() -> Self {
        Self::new()
    }
}
