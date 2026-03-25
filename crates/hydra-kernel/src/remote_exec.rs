//! Remote execution bridge — thin wrapper for SSH commands from conductor.
//! Reads machine config from ~/.hydra/machines.toml, executes via `ssh` command.
//! Keeps hydra-kernel independent of hydra-executor (Law 8: dependency direction).

use serde::Deserialize;

/// Execute a command on a named remote machine via SSH.
/// Looks up machine in ~/.hydra/machines.toml, builds SSH command, executes.
pub fn ssh_execute(machine_name: &str, command: &str) -> Result<(String, bool), String> {
    let machine = load_machine(machine_name)?;
    let mut args = vec![
        "-o".to_string(), "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(), "ConnectTimeout=10".to_string(),
        "-p".to_string(), machine.port.to_string(),
    ];
    if let Some(key) = &machine.key_path {
        args.push("-i".to_string());
        args.push(key.clone());
    }
    args.push(format!("{}@{}", machine.user, machine.host));
    // EC-24.4: Shell-escape to prevent command injection via metacharacters
    args.push(format!("'{}'", command.replace('\'', "'\\''")));

    eprintln!("hydra-remote: ssh {}@{}:{} '{}'", machine.user, machine.host, machine.port, command);
    let output = std::process::Command::new("ssh").args(&args).output()
        .map_err(|e| format!("SSH command failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = if stderr.is_empty() { stdout } else { format!("{stdout}\n{stderr}") };
    Ok((combined, output.status.success()))
}

// ── Machine Config ──

#[derive(Deserialize)]
struct MachineEntry {
    name: String,
    host: String,
    #[serde(default = "default_user")]
    user: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    key_path: Option<String>,
}

fn default_user() -> String { "root".into() }
fn default_port() -> u16 { 22 }

#[derive(Deserialize)]
struct MachinesFile { #[serde(default)] machine: Vec<MachineEntry> }

fn load_machine(name: &str) -> Result<MachineEntry, String> {
    let path = dirs::home_dir().unwrap_or_default().join(".hydra/machines.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| format!("No machines.toml found. Create ~/.hydra/machines.toml"))?;
    let file: MachinesFile = toml::from_str(&content)
        .map_err(|e| format!("Parse error in machines.toml: {e}"))?;
    file.machine.into_iter().find(|m| m.name == name)
        .ok_or_else(|| format!("Machine '{name}' not found in machines.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_machine_returns_error() {
        let r = load_machine("nonexistent_xyz_machine");
        assert!(r.is_err());
    }

    #[test]
    fn default_port_is_22() {
        assert_eq!(default_port(), 22);
    }
}
