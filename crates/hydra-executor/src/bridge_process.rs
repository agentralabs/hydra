//! BridgeProcess — manages one bridge subprocess lifecycle.
//! Follows hydra-voice pattern: spawn + background reader thread + AtomicBool stop.

use crate::bridge_config::BridgeConfig;
use crate::constants::{BRIDGE_RESTART_BACKOFF_BASE_MS, BRIDGE_RESTART_BACKOFF_MAX_MS};
use crate::runtime::read_credentials;

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// State of a bridge subprocess.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BridgeState {
    Starting,
    Running,
    Unhealthy,
    Stopped,
    Crashed { restart_in_ms: u64 },
}

/// A signal emitted by a bridge process.
#[derive(Debug, Clone)]
pub enum BridgeSignal {
    Message { bridge_name: String, content: String, sender: String },
    Connected { bridge_name: String },
    Disconnected { bridge_name: String, reason: String },
    HealthFailed { bridge_name: String },
}

/// One bridge subprocess with its reader thread.
pub struct BridgeProcess {
    pub name: String,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    reader_thread: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    pub state: BridgeState,
    pub config: BridgeConfig,
    pub restart_count: u32,
    pub last_health: Option<Instant>,
    pub started_at: Option<Instant>,
}

impl BridgeProcess {
    /// Spawn the bridge subprocess and start the reader thread.
    pub fn spawn(
        name: &str,
        config: &BridgeConfig,
        integration_dir: &Path,
        signal_tx: mpsc::Sender<BridgeSignal>,
    ) -> Result<Self, String> {
        let runtime = &config.bridge.runtime;
        let entry = integration_dir.join(&config.bridge.entry);

        if !entry.exists() {
            return Err(format!(
                "Bridge entry point not found: {}",
                entry.display()
            ));
        }

        // Build the command
        let (cmd_name, mut cmd_args) = match runtime.as_str() {
            "node" => ("node", vec![entry.to_string_lossy().to_string()]),
            "python" => ("python3", vec![entry.to_string_lossy().to_string()]),
            "binary" => (entry.to_str().unwrap_or(""), vec![]),
            "script" => ("sh", vec![entry.to_string_lossy().to_string()]),
            other => return Err(format!("Unknown runtime: {other}")),
        };

        for arg in &config.bridge.args {
            cmd_args.push(arg.clone());
        }

        let mut command = Command::new(cmd_name);
        command
            .args(&cmd_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(integration_dir);

        // Inject vault credentials as env vars
        if let Some(vault_key) = &config.credentials.vault_service {
            let creds = read_credentials(vault_key);
            for (k, v) in &creds {
                command.env(k.to_uppercase(), v);
            }
        }
        for env_var in &config.credentials.env_vars {
            if let Ok(val) = std::env::var(env_var) {
                command.env(env_var, val);
            }
        }

        let mut child = command.spawn().map_err(|e| {
            format!("Failed to spawn bridge '{name}': {e}")
        })?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();

        // Start background reader thread
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let bridge_name = name.to_string();
        let msg_field = config.bridge.incoming.message_field.clone();
        let sender_field = config.bridge.incoming.sender_field.clone();

        let reader_thread = stdout.map(|stdout| {
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    if line.trim().is_empty() {
                        continue;
                    }
                    // Parse JSON line
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                        let content = val.get(&msg_field)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let sender = val.get(&sender_field)
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        if !content.is_empty() {
                            let _ = signal_tx.send(BridgeSignal::Message {
                                bridge_name: bridge_name.clone(),
                                content,
                                sender,
                            });
                        }
                    }
                }
            })
        });

        eprintln!("hydra-executor: bridge '{name}' spawned (pid={})", child.id());

        Ok(Self {
            name: name.to_string(),
            child: Some(child),
            stdin,
            reader_thread,
            stop,
            state: BridgeState::Running,
            config: config.clone(),
            restart_count: 0,
            last_health: Some(Instant::now()),
            started_at: Some(Instant::now()),
        })
    }

    /// Send a JSON line to the bridge's stdin.
    pub fn send(&mut self, message: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("Bridge stdin not available")?;
        writeln!(stdin, "{message}").map_err(|e| format!("Write to bridge: {e}"))?;
        stdin.flush().map_err(|e| format!("Flush bridge stdin: {e}"))?;
        Ok(())
    }

    /// Check if the process is still alive.
    pub fn is_alive(&mut self) -> bool {
        if let Some(child) = &mut self.child {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }

    /// Kill the subprocess and join the reader thread.
    pub fn kill(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(mut child) = self.child.take() {
            // Send shutdown command first
            if let Some(stdin) = &mut self.stdin {
                let _ = writeln!(stdin, "{}", self.config.bridge.lifecycle.shutdown_command);
                let _ = stdin.flush();
            }
            // Brief wait, then force kill
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        if let Some(thread) = self.reader_thread.take() {
            let _ = thread.join();
        }
        self.state = BridgeState::Stopped;
        eprintln!("hydra-executor: bridge '{}' stopped", self.name);
    }

    /// Calculate backoff delay for restart.
    pub fn restart_backoff_ms(&self) -> u64 {
        let delay = BRIDGE_RESTART_BACKOFF_BASE_MS * 2u64.pow(self.restart_count);
        delay.min(BRIDGE_RESTART_BACKOFF_MAX_MS)
    }
}

impl Drop for BridgeProcess {
    fn drop(&mut self) {
        if self.child.is_some() {
            self.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_state_serialization() {
        let state = BridgeState::Running;
        let json = serde_json::to_string(&state).unwrap();
        let back: BridgeState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, BridgeState::Running);
    }

    #[test]
    fn backoff_increases_exponentially() {
        let proc = BridgeProcess {
            name: "test".into(),
            child: None,
            stdin: None,
            reader_thread: None,
            stop: Arc::new(AtomicBool::new(false)),
            state: BridgeState::Stopped,
            config: make_test_config(),
            restart_count: 0,
            last_health: None,
            started_at: None,
        };
        assert_eq!(proc.restart_backoff_ms(), 1_000);

        let mut proc2 = proc;
        proc2.restart_count = 3;
        assert_eq!(proc2.restart_backoff_ms(), 8_000);

        proc2.restart_count = 10;
        assert_eq!(proc2.restart_backoff_ms(), BRIDGE_RESTART_BACKOFF_MAX_MS);
    }

    #[test]
    fn crashed_state_serialization() {
        let state = BridgeState::Crashed { restart_in_ms: 5000 };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("5000"));
    }

    fn make_test_config() -> BridgeConfig {
        toml::from_str(r#"
[integration]
name = "test"
[bridge]
runtime = "node"
entry = "test.js"
"#).unwrap()
    }
}
