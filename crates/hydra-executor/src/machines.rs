//! Machine Registry — tracks remote machines Hydra has access to.
//! Parsed from ~/.hydra/machines.toml. Lookup by name for SSH execution.

use crate::remote::{ExecutionTarget, RemoteAuth};
use serde::Deserialize;
use std::path::PathBuf;

/// A registered remote machine.
#[derive(Debug, Clone)]
pub struct Machine {
    pub name: String,
    pub host: String,
    pub user: String,
    pub auth: RemoteAuth,
    pub port: u16,
    pub capabilities: Vec<String>,
    pub health_endpoint: Option<String>,
}

/// Registry of all configured remote machines.
pub struct MachineRegistry {
    machines: Vec<Machine>,
}

impl MachineRegistry {
    /// Load machine registry from ~/.hydra/machines.toml.
    pub fn load() -> Self {
        let path = dirs::home_dir().unwrap_or_default().join(".hydra/machines.toml");
        let machines = if let Ok(content) = std::fs::read_to_string(&path) {
            parse_machines(&content)
        } else {
            Vec::new()
        };
        eprintln!("hydra-machines: loaded {} machines", machines.len());
        Self { machines }
    }

    /// Create empty registry (for testing).
    pub fn empty() -> Self { Self { machines: Vec::new() } }

    /// Lookup a machine by name.
    pub fn lookup(&self, name: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.name == name)
    }

    /// List all machines.
    pub fn list(&self) -> &[Machine] { &self.machines }

    /// Number of machines.
    pub fn count(&self) -> usize { self.machines.len() }
}

/// Convert a Machine to an ExecutionTarget for SSH execution.
pub fn to_target(machine: &Machine) -> ExecutionTarget {
    ExecutionTarget::Remote {
        host: machine.host.clone(),
        user: machine.user.clone(),
        auth: machine.auth.clone(),
        port: machine.port,
    }
}

// ── TOML Parsing ──

#[derive(Deserialize)]
struct MachinesFile {
    #[serde(default)]
    machine: Vec<MachineEntry>,
}

#[derive(Deserialize)]
struct MachineEntry {
    name: String,
    host: String,
    #[serde(default = "default_user")]
    user: String,
    #[serde(default = "default_auth")]
    auth: String,
    #[serde(default)]
    key_path: Option<String>,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    health_endpoint: Option<String>,
}

fn default_user() -> String { "root".into() }
fn default_auth() -> String { "ssh_agent".into() }
fn default_port() -> u16 { 22 }

fn parse_machines(content: &str) -> Vec<Machine> {
    let file: MachinesFile = match toml::from_str(content) {
        Ok(f) => f,
        Err(e) => { eprintln!("hydra-machines: parse error: {e}"); return Vec::new(); }
    };
    file.machine.into_iter().map(|entry| {
        let auth = match entry.auth.as_str() {
            "ssh_key" => RemoteAuth::SshKey {
                path: PathBuf::from(entry.key_path.unwrap_or_else(|| "~/.ssh/id_rsa".into())),
            },
            "password" => RemoteAuth::Password { vault_key: format!("ssh_{}", entry.name) },
            _ => RemoteAuth::SshAgent,
        };
        Machine {
            name: entry.name, host: entry.host, user: entry.user,
            auth, port: entry.port, capabilities: entry.capabilities,
            health_endpoint: entry.health_endpoint,
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_loads() {
        let reg = MachineRegistry::empty();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn lookup_returns_none_for_missing() {
        let reg = MachineRegistry::empty();
        assert!(reg.lookup("nonexistent").is_none());
    }

    #[test]
    fn parse_toml() {
        let toml = r#"
[[machine]]
name = "prod"
host = "prod.example.com"
user = "deploy"
auth = "ssh_agent"
port = 22
capabilities = ["shell", "docker"]
"#;
        let machines = parse_machines(toml);
        assert_eq!(machines.len(), 1);
        assert_eq!(machines[0].name, "prod");
        assert_eq!(machines[0].host, "prod.example.com");
    }

    #[test]
    fn machine_to_target() {
        let m = Machine {
            name: "test".into(), host: "host.com".into(), user: "usr".into(),
            auth: RemoteAuth::SshAgent, port: 22, capabilities: vec![],
            health_endpoint: None,
        };
        let target = to_target(&m);
        assert!(matches!(target, ExecutionTarget::Remote { .. }));
    }
}
