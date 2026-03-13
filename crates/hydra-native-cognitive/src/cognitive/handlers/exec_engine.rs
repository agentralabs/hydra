//! Execution engine — persistent shell state, parallel execution, background processes.
//!
//! Gives Hydra full freedom: working directory carries across commands, env vars persist,
//! independent commands run in parallel, and long-running processes run in background.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Output;
use std::sync::{Arc, OnceLock};
use parking_lot::Mutex;
use tokio::sync::mpsc;

use super::super::loop_runner::CognitiveUpdate;
use super::llm_helpers::commands_are_dependent;

/// Persistent shell state that carries across command executions within a session.
#[derive(Debug, Clone)]
pub struct ShellState {
    pub cwd: PathBuf,
    pub env_vars: HashMap<String, String>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
            }),
            env_vars: HashMap::new(),
        }
    }
}

fn shell_state() -> &'static Arc<Mutex<ShellState>> {
    static STATE: OnceLock<Arc<Mutex<ShellState>>> = OnceLock::new();
    STATE.get_or_init(|| Arc::new(Mutex::new(ShellState::default())))
}

fn bg_processes() -> &'static Arc<Mutex<Vec<BackgroundProcess>>> {
    static PROCS: OnceLock<Arc<Mutex<Vec<BackgroundProcess>>>> = OnceLock::new();
    PROCS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

/// Background process tracker.
pub struct BackgroundProcess {
    pub id: String,
    pub command: String,
    pub handle: tokio::task::JoinHandle<Option<Output>>,
}

/// Run a command with persistent shell state (cwd + env vars).
pub async fn run_command(cmd: &str) -> std::io::Result<Output> {
    let (cwd, env_vars) = {
        let state = shell_state().lock();
        (state.cwd.clone(), state.env_vars.clone())
    };
    let wrapped = format!(
        "cd {:?} 2>/dev/null; {}", cwd.display(), cmd
    );
    let mut command = tokio::process::Command::new("sh");
    command.arg("-c").arg(&wrapped);
    for (k, v) in &env_vars {
        command.env(k, v);
    }
    let output = command.output().await?;
    // Update persistent state from command
    update_cwd_from_command(cmd, &cwd);
    capture_env_exports(cmd);
    Ok(output)
}

/// Update working directory if command contains `cd`.
fn update_cwd_from_command(cmd: &str, current_cwd: &PathBuf) {
    let trimmed = cmd.trim();
    let cd_targets: Vec<&str> = trimmed.split("&&").chain(trimmed.split(";"))
        .filter_map(|part| {
            let p = part.trim();
            if p.starts_with("cd ") {
                Some(p.strip_prefix("cd ").unwrap().trim().trim_matches('"').trim_matches('\''))
            } else { None }
        })
        .collect();
    if let Some(target) = cd_targets.last() {
        let new_path = if target.starts_with('/') {
            PathBuf::from(target)
        } else if target.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(target.replacen('~', &home, 1))
        } else if *target == ".." {
            current_cwd.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| current_cwd.clone())
        } else {
            current_cwd.join(target)
        };
        if new_path.is_dir() {
            shell_state().lock().cwd = new_path;
        }
    }
}

/// Capture `export KEY=VALUE` from command text to persist env vars.
fn capture_env_exports(cmd: &str) {
    for part in cmd.split("&&").chain(cmd.split(";")) {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("export ") {
            if let Some((key, val)) = rest.split_once('=') {
                let k = key.trim().to_string();
                let v = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !k.is_empty() {
                    shell_state().lock().env_vars.insert(k, v);
                }
            }
        }
    }
}

/// Partition commands into parallel groups. Independent commands share a group.
pub fn partition_parallel(commands: &[String]) -> Vec<Vec<usize>> {
    if commands.len() <= 1 {
        return commands.iter().enumerate().map(|(i, _)| vec![i]).collect();
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = vec![0];
    for i in 1..commands.len() {
        let depends = current_group.iter().any(|&j| {
            commands_are_dependent(&commands[j], &commands[i])
        });
        if depends {
            groups.push(current_group);
            current_group = vec![i];
        } else {
            current_group.push(i);
        }
    }
    if !current_group.is_empty() { groups.push(current_group); }
    groups
}

/// Execute multiple independent commands in parallel via tokio::spawn.
pub async fn run_parallel(commands: &[String]) -> Vec<std::io::Result<Output>> {
    let handles: Vec<_> = commands.iter().map(|cmd| {
        let c = cmd.clone();
        tokio::spawn(async move { run_command(&c).await })
    }).collect();
    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok(r) => results.push(r),
            Err(e) => results.push(Err(std::io::Error::new(std::io::ErrorKind::Other, e))),
        }
    }
    results
}

/// Launch a command as a background process. Returns a short ID.
pub fn spawn_background(cmd: &str, tx: &mpsc::UnboundedSender<CognitiveUpdate>) -> String {
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let cmd_owned = cmd.to_string();
    let tx_clone = tx.clone();
    let id_clone = id.clone();
    let handle = tokio::spawn(async move {
        let result = run_command(&cmd_owned).await.ok();
        let success = result.as_ref().map(|o| o.status.success()).unwrap_or(false);
        let _ = tx_clone.send(CognitiveUpdate::EvidenceMemory {
            title: format!("Background [{}] done", id_clone),
            content: format!("{} | success={}", cmd_owned, success),
        });
        result
    });
    bg_processes().lock().push(BackgroundProcess { id: id.clone(), command: cmd.to_string(), handle });
    let _ = tx.send(CognitiveUpdate::Phase(format!("Background [{}]: {}", id, cmd)));
    eprintln!("[hydra:bg] Spawned [{}]: {}", id, cmd);
    id
}

/// List active background processes: (id, command, still_running).
pub fn list_background() -> Vec<(String, String, bool)> {
    bg_processes().lock().iter().map(|p| {
        (p.id.clone(), p.command.clone(), !p.handle.is_finished())
    }).collect()
}

/// Get the current working directory.
pub fn current_cwd() -> PathBuf { shell_state().lock().cwd.clone() }

/// Get current env vars.
pub fn current_env() -> HashMap<String, String> { shell_state().lock().env_vars.clone() }

/// Reset shell state (new session).
pub fn reset_shell_state() { *shell_state().lock() = ShellState::default(); }

/// Detect if a command should auto-run in background (long-running services).
pub fn should_background(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    if lower.ends_with(" &") || lower.contains("nohup ") { return true; }
    let patterns = [
        "npm run dev", "npm start", "yarn dev", "yarn start",
        "python -m http.server", "python3 -m http.server",
        "cargo run", "cargo watch", "docker compose up", "docker-compose up",
        "serve ", "http-server", "live-server", "tail -f", "watch ",
        "ngrok", "localtunnel",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_state_default() {
        let state = ShellState::default();
        assert!(state.cwd.exists() || state.cwd == PathBuf::from("/tmp"));
        assert!(state.env_vars.is_empty());
    }

    #[test]
    fn test_capture_env_exports() {
        let unique = format!("HYDRA_TEST_{}", std::process::id());
        capture_env_exports(&format!("export {}=hello", unique));
        let env = current_env();
        assert_eq!(env.get(&unique).map(|s| s.as_str()), Some("hello"));
    }

    #[test]
    fn test_update_cwd_absolute() {
        let cwd = PathBuf::from("/tmp");
        update_cwd_from_command("cd /tmp", &cwd);
        // Just verify it doesn't panic — cwd update depends on /tmp existing
    }

    #[test]
    fn test_partition_single() {
        let cmds = vec!["ls".to_string()];
        assert_eq!(partition_parallel(&cmds), vec![vec![0]]);
    }

    #[test]
    fn test_should_background() {
        assert!(should_background("npm run dev"));
        assert!(should_background("cargo watch -x test"));
        assert!(should_background("python3 -m http.server 8080"));
        assert!(should_background("sleep 100 &"));
        assert!(!should_background("ls -la"));
        assert!(!should_background("cargo build"));
        assert!(!should_background("npm install"));
    }

    #[test]
    fn test_partition_independent() {
        let cmds = vec!["ls -la".into(), "echo hello".into(), "date".into()];
        let groups = partition_parallel(&cmds);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0], vec![0, 1, 2]);
    }

    #[test]
    fn test_list_background_no_panic() {
        let _ = list_background();
    }

    #[test]
    fn test_current_cwd_exists() {
        let cwd = current_cwd();
        // Should return something valid
        assert!(!cwd.as_os_str().is_empty());
    }
}
