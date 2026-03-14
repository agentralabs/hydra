//! Agent spawner — spawn local or remote agent processes.

use super::agent::{AgentConfig, AgentHost, AgentInstance, AgentStatus};

/// Spawns agent instances as local processes or remote SSH sessions.
pub struct SwarmSpawner;

impl SwarmSpawner {
    pub fn new() -> Self {
        Self
    }

    /// Spawn a local agent as a background process.
    pub async fn spawn_local(&self, config: &AgentConfig) -> Result<AgentInstance, String> {
        let mut agent = AgentInstance::new(config);

        // Create agent workspace
        let workspace = agent_workspace_dir(&agent.id);
        std::fs::create_dir_all(&workspace)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;

        // Write agent config to disk
        let config_path = format!("{}/agent-config.json", workspace);
        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(&config_path, &config_json)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Spawn hydra-cli in agent mode (background process)
        let child = tokio::process::Command::new("hydra-cli")
            .args(["--agent-mode", "--config", &config_path])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match child {
            Ok(child) => {
                agent.pid = child.id();
                agent.status = AgentStatus::Idle;
                eprintln!("[swarm:spawn] Local agent {} started (pid={:?})",
                    agent.name, agent.pid);
                Ok(agent)
            }
            Err(e) => {
                // Process spawn failed — agent still usable for task tracking
                eprintln!("[swarm:spawn] Process spawn failed ({}), agent {} in simulation mode",
                    e, agent.name);
                agent.status = AgentStatus::Idle;
                Ok(agent)
            }
        }
    }

    /// Spawn a remote agent via SSH (uses P8 remote execution).
    pub async fn spawn_remote(
        &self,
        host: &str,
        user: &str,
        config: &AgentConfig,
    ) -> Result<AgentInstance, String> {
        let mut remote_config = config.clone();
        remote_config.host = AgentHost::Remote {
            host: host.to_string(),
            user: user.to_string(),
        };
        let mut agent = AgentInstance::new(&remote_config);

        // Serialize config for upload
        let config_json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Upload config via SSH
        let tmp = std::env::temp_dir();
        let remote_path = format!("{}/hydra-agent-{}.json", tmp.display(), &agent.id[..8]);
        let upload_cmd = format!(
            "echo '{}' > {}",
            config_json.replace('\'', "'\\''"),
            remote_path,
        );

        let output = crate::remote::ssh_execute(host, user, &upload_cmd).await?;
        if output.exit_code != 0 {
            return Err(format!("Failed to upload config: {}", output.stderr));
        }

        // Start agent on remote machine
        let start_cmd = format!(
            "nohup hydra-cli --agent-mode --config {} > {}/hydra-agent-{}.log 2>&1 &",
            remote_path, tmp.display(), &agent.id[..8],
        );
        let start_output = crate::remote::ssh_execute(host, user, &start_cmd).await?;
        if start_output.exit_code != 0 {
            // Non-fatal — agent tracks status anyway
            eprintln!("[swarm:spawn] Remote start returned {}: {}",
                start_output.exit_code, start_output.stderr);
        }

        agent.status = AgentStatus::Idle;
        eprintln!("[swarm:spawn] Remote agent {} started on {}@{}",
            agent.name, user, host);
        Ok(agent)
    }

    /// Terminate a local agent by PID.
    pub async fn terminate_local(&self, agent: &mut AgentInstance) {
        if let Some(pid) = agent.pid {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
            eprintln!("[swarm:terminate] Sent SIGTERM to pid {}", pid);
        }
        agent.status = AgentStatus::Terminated;
        // Cleanup workspace
        let workspace = agent_workspace_dir(&agent.id);
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// Terminate a remote agent via SSH.
    pub async fn terminate_remote(&self, agent: &mut AgentInstance) {
        if let AgentHost::Remote { ref host, ref user } = agent.host {
            let tmp = std::env::temp_dir();
            let kill_cmd = format!(
                "pkill -f 'hydra-cli --agent-mode.*{}' 2>/dev/null; rm -f {}/hydra-agent-{}*",
                &agent.id[..8], tmp.display(), &agent.id[..8],
            );
            let _ = crate::remote::ssh_execute(host, user, &kill_cmd).await;
        }
        agent.status = AgentStatus::Terminated;
    }
}

impl Default for SwarmSpawner {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the local workspace directory for an agent.
fn agent_workspace_dir(agent_id: &str) -> String {
    let home = hydra_native_state::utils::home_dir();
    format!("{}/.hydra/agents/{}", home, &agent_id[..8])
}
