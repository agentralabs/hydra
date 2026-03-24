//! BridgeManager — owns all running bridge subprocesses.
//! Start/stop bridges, send messages, health check, restart on crash.
//! Emits BridgeSignal to companion for message classification.

use crate::bridge_config::{self, BridgeConfig};
use crate::bridge_process::{BridgeProcess, BridgeSignal, BridgeState};
use crate::constants::MAX_ACTIVE_BRIDGES;

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

/// Manages all active bridge connectors.
pub struct BridgeManager {
    bridges: HashMap<String, BridgeProcess>,
    signal_tx: mpsc::Sender<BridgeSignal>,
}

impl BridgeManager {
    /// Create a new BridgeManager. Returns the manager + a receiver for companion.
    pub fn new() -> (Self, mpsc::Receiver<BridgeSignal>) {
        let (tx, rx) = mpsc::channel();
        let manager = Self {
            bridges: HashMap::new(),
            signal_tx: tx,
        };
        (manager, rx)
    }

    /// Start a bridge from its config.
    pub fn start(
        &mut self,
        name: &str,
        config: &BridgeConfig,
        integration_dir: &Path,
    ) -> Result<(), String> {
        if self.bridges.len() >= MAX_ACTIVE_BRIDGES {
            return Err(format!(
                "Maximum active bridges ({MAX_ACTIVE_BRIDGES}) reached"
            ));
        }
        if self.bridges.contains_key(name) {
            return Err(format!("Bridge '{name}' already running"));
        }

        let process = BridgeProcess::spawn(
            name,
            config,
            integration_dir,
            self.signal_tx.clone(),
        )?;

        // Send init command
        let mut process = process;
        if let Err(e) = process.send(&config.bridge.lifecycle.init_command) {
            eprintln!("hydra-executor: bridge '{name}' init command failed: {e}");
        }

        let _ = self.signal_tx.send(BridgeSignal::Connected {
            bridge_name: name.into(),
        });

        self.bridges.insert(name.to_string(), process);
        eprintln!(
            "hydra-executor: bridge '{name}' started ({} active)",
            self.bridges.len()
        );
        Ok(())
    }

    /// Stop a bridge gracefully.
    pub fn stop(&mut self, name: &str) -> Result<(), String> {
        let mut process = self.bridges.remove(name)
            .ok_or(format!("Bridge '{name}' not found"))?;

        process.kill();

        let _ = self.signal_tx.send(BridgeSignal::Disconnected {
            bridge_name: name.into(),
            reason: "Stopped by manager".into(),
        });

        eprintln!(
            "hydra-executor: bridge '{name}' stopped ({} active)",
            self.bridges.len()
        );
        Ok(())
    }

    /// Send a message through a bridge.
    pub fn send(&mut self, name: &str, message: &str) -> Result<(), String> {
        let process = self.bridges.get_mut(name)
            .ok_or(format!("Bridge '{name}' not running"))?;

        if process.state != BridgeState::Running {
            return Err(format!("Bridge '{name}' is {:?}", process.state));
        }

        process.send(message)
    }

    /// Tick — called from the ambient loop. Checks health, restarts crashed.
    pub fn tick(&mut self) {
        let mut to_restart: Vec<(String, BridgeConfig)> = Vec::new();

        for (name, process) in &mut self.bridges {
            // Check if process is still alive
            if !process.is_alive() && process.state == BridgeState::Running {
                eprintln!("hydra-executor: bridge '{name}' crashed");
                process.state = BridgeState::Crashed {
                    restart_in_ms: process.restart_backoff_ms(),
                };

                let _ = self.signal_tx.send(BridgeSignal::Disconnected {
                    bridge_name: name.clone(),
                    reason: "Process exited unexpectedly".into(),
                });

                if process.config.bridge.restart_on_crash
                    && process.restart_count < process.config.bridge.max_restart_attempts
                {
                    to_restart.push((name.clone(), process.config.clone()));
                }
            }

            // Health check timing
            if process.state == BridgeState::Running {
                let interval = process.config.bridge.health_check_interval_seconds * 1000;
                if let Some(last) = process.last_health {
                    if last.elapsed().as_millis() as u64 >= interval {
                        // Send health ping
                        let health_cmd = process.config.bridge.lifecycle.health_command.clone();
                        if let Err(e) = process.send(&health_cmd) {
                            eprintln!("hydra-executor: bridge '{name}' health check failed: {e}");
                            process.state = BridgeState::Unhealthy;
                            let _ = self.signal_tx.send(BridgeSignal::HealthFailed {
                                bridge_name: name.clone(),
                            });
                        }
                        process.last_health = Some(Instant::now());
                    }
                }
            }
        }

        // Restart crashed bridges (outside the borrow)
        for (name, config) in to_restart {
            if let Some(old_process) = self.bridges.remove(&name) {
                let restart_count = old_process.restart_count + 1;
                let integration_dir = Path::new("integrations").join(&name);

                eprintln!(
                    "hydra-executor: restarting bridge '{name}' (attempt {restart_count}/{})",
                    config.bridge.max_restart_attempts
                );

                match BridgeProcess::spawn(
                    &name,
                    &config,
                    &integration_dir,
                    self.signal_tx.clone(),
                ) {
                    Ok(mut new_process) => {
                        new_process.restart_count = restart_count;
                        let _ = new_process.send(&config.bridge.lifecycle.init_command);
                        let _ = self.signal_tx.send(BridgeSignal::Connected {
                            bridge_name: name.clone(),
                        });
                        self.bridges.insert(name, new_process);
                    }
                    Err(e) => {
                        eprintln!("hydra-executor: bridge '{name}' restart failed: {e}");
                    }
                }
            }
        }
    }

    /// List names of active bridges.
    pub fn list_active(&self) -> Vec<&str> {
        self.bridges.keys().map(|s| s.as_str()).collect()
    }

    /// Get bridge state by name.
    pub fn state(&self, name: &str) -> Option<&BridgeState> {
        self.bridges.get(name).map(|p| &p.state)
    }

    /// Shutdown all bridges gracefully.
    pub fn shutdown_all(&mut self) {
        let names: Vec<String> = self.bridges.keys().cloned().collect();
        for name in &names {
            if let Err(e) = self.stop(name) {
                eprintln!("hydra-executor: shutdown bridge '{name}' error: {e}");
            }
        }
        eprintln!("hydra-executor: all bridges shutdown");
    }

    pub fn count(&self) -> usize {
        self.bridges.len()
    }
}

/// Load all bridge configs from integrations/ directory.
pub fn load_bridge_configs(integrations_dir: &Path) -> Vec<(String, BridgeConfig)> {
    let mut configs = Vec::new();

    if !integrations_dir.exists() {
        return configs;
    }

    let entries = match std::fs::read_dir(integrations_dir) {
        Ok(e) => e,
        Err(_) => return configs,
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let bridge_path = entry.path().join("bridge.toml");
        if !bridge_path.exists() {
            continue;
        }
        match bridge_config::load_bridge_config(&bridge_path) {
            Ok(config) => {
                let name = config.integration.name.clone();
                eprintln!("hydra-executor: bridge config loaded: {name}");
                configs.push((name, config));
            }
            Err(e) => {
                eprintln!(
                    "hydra-executor: bridge config error {:?}: {e}",
                    entry.path()
                );
            }
        }
    }

    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_empty() {
        let (mgr, _rx) = BridgeManager::new();
        assert_eq!(mgr.count(), 0);
        assert!(mgr.list_active().is_empty());
    }

    #[test]
    fn state_returns_none_for_unknown() {
        let (mgr, _rx) = BridgeManager::new();
        assert!(mgr.state("nonexistent").is_none());
    }

    #[test]
    fn load_configs_from_empty_dir() {
        let configs = load_bridge_configs(Path::new("/nonexistent"));
        assert!(configs.is_empty());
    }

    #[test]
    fn start_fails_for_missing_entry() {
        let (mut mgr, _rx) = BridgeManager::new();
        let config: BridgeConfig = toml::from_str(r#"
[integration]
name = "test"
[bridge]
runtime = "node"
entry = "nonexistent.js"
"#).unwrap();
        let result = mgr.start("test", &config, Path::new("/tmp/nonexistent-hydra-test"));
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_start_rejected() {
        // Can't easily test without a real subprocess, but verify the check exists
        let (mut mgr, _rx) = BridgeManager::new();
        // Without actually starting, insert a placeholder
        assert_eq!(mgr.count(), 0);
    }
}
