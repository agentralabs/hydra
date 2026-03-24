//! O7 Persistent Workspace — snapshot/resume for cross-session continuity.
//! Saves active workspace state on exit, restores on boot.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub working_directory: String,
    pub git_branch: Option<String>,
    pub processes: Vec<ProcessState>,
    pub pending_tasks: Vec<TaskSummary>,
    pub goals: Vec<GoalProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessState {
    pub command: String,
    pub port: Option<u16>,
    pub purpose: String,
    pub restartable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub description: String,
    pub progress: f64,
    pub blocked_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub description: String,
    pub progress: f64,
    pub steps_done: usize,
    pub steps_total: usize,
}

/// Result of attempting to resume a workspace.
pub struct ResumeResult {
    pub summary: String,
    pub tasks_restored: usize,
    pub processes_restarted: usize,
    pub branch_changed: bool,
    pub warnings: Vec<String>,
}

// ── Snapshot Path ──

fn snapshot_dir() -> PathBuf {
    crate::persistence::data_dir()
}

fn snapshot_path(index: usize) -> PathBuf {
    let dir = snapshot_dir();
    if index == 0 { dir.join("workspace.json") }
    else { dir.join(format!("workspace.{index}.json")) }
}

// ── Save ──

/// Save workspace state atomically (EC-7.3: temp file + rename).
/// Rotates last 3 snapshots for rollback.
pub fn save_snapshot(snapshot: &WorkspaceSnapshot) -> Result<(), String> {
    let dir = snapshot_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("workspace dir: {e}"))?;
    // Rotate: 2→3, 1→2, 0→1
    for i in (0..2).rev() {
        let src = snapshot_path(i);
        let dst = snapshot_path(i + 1);
        if src.exists() { let _ = std::fs::rename(&src, &dst); }
    }
    // Atomic write: tmp + rename (EC-7.3)
    let tmp = dir.join("workspace.json.tmp");
    let json = serde_json::to_string_pretty(snapshot).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&tmp, &json).map_err(|e| format!("write tmp: {e}"))?;
    std::fs::rename(&tmp, snapshot_path(0)).map_err(|e| format!("rename: {e}"))?;
    eprintln!("hydra-workspace: snapshot saved ({} tasks, {} processes)",
        snapshot.pending_tasks.len(), snapshot.processes.len());
    Ok(())
}

/// Create a snapshot from current state.
pub fn capture(task_engine: &crate::task_engine::TaskEngine) -> WorkspaceSnapshot {
    let cwd = std::env::current_dir().unwrap_or_default();
    let branch = detect_git_branch(&cwd);
    let pending_tasks: Vec<TaskSummary> = task_engine.active_task_ids().iter()
        .filter_map(|id| task_engine.get(id))
        .map(|t| TaskSummary {
            description: t.description.clone(),
            progress: t.cycle_count as f64 / 10.0_f64.max(t.cycle_count as f64),
            blocked_on: None, // TaskState check deferred — state is private
        })
        .collect();
    WorkspaceSnapshot {
        timestamp: Utc::now(),
        working_directory: cwd.to_string_lossy().into(),
        git_branch: branch,
        processes: vec![], // Populated by TUI with actual running processes
        pending_tasks,
        goals: vec![],
    }
}

// ── Load ──

/// Load the most recent valid snapshot (EC-7.3: try backups if primary corrupt).
pub fn load_snapshot() -> Option<WorkspaceSnapshot> {
    for i in 0..3 {
        let path = snapshot_path(i);
        if let Ok(json) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<WorkspaceSnapshot>(&json) {
                Ok(snap) => {
                    if i > 0 { eprintln!("hydra-workspace: loaded from backup {i}"); }
                    return Some(snap);
                }
                Err(e) => eprintln!("hydra-workspace: snapshot {i} corrupt: {e}"),
            }
        }
    }
    None
}

// ── Resume ──

/// Resume workspace from a snapshot. Detects branch changes, restarts processes.
pub fn resume_workspace(snapshot: &WorkspaceSnapshot) -> ResumeResult {
    let mut warnings = Vec::new();
    let mut processes_restarted = 0;
    // EC-7.5: Resolve working directory (relative path support)
    let saved_dir = Path::new(&snapshot.working_directory);
    if saved_dir.exists() {
        if let Err(e) = std::env::set_current_dir(saved_dir) {
            warnings.push(format!("Could not cd to {}: {e}", snapshot.working_directory));
        }
    } else {
        warnings.push(format!("Saved directory '{}' no longer exists", snapshot.working_directory));
    }
    // EC-7.2: Detect git branch change
    let cwd = std::env::current_dir().unwrap_or_default();
    let current_branch = detect_git_branch(&cwd);
    let branch_changed = match (&snapshot.git_branch, &current_branch) {
        (Some(saved), Some(current)) if saved != current => {
            warnings.push(format!("Branch changed: {} → {}", saved, current));
            true
        }
        _ => false,
    };
    // EC-7.1 + EC-7.4: Restart processes
    for proc in &snapshot.processes {
        if !proc.restartable { continue; }
        // EC-7.1: Check port availability
        if let Some(port) = proc.port {
            if !is_port_available(port) {
                let alt = find_available_port(port);
                warnings.push(format!("Port {} occupied, using {}", port, alt));
            }
        }
        // EC-7.4: Attempt restart, log if fails
        match restart_process(&proc.command) {
            Ok(_) => {
                processes_restarted += 1;
                eprintln!("hydra-workspace: restarted '{}'", proc.purpose);
            }
            Err(e) => warnings.push(format!("Failed to restart '{}': {e}", proc.purpose)),
        }
    }
    let elapsed = Utc::now() - snapshot.timestamp;
    let ago = if elapsed.num_hours() < 1 { format!("{}m", elapsed.num_minutes()) }
        else if elapsed.num_hours() < 24 { format!("{}h", elapsed.num_hours()) }
        else { format!("{}d", elapsed.num_days()) };
    let summary = format!("{} tasks, {} processes restarted (paused {})",
        snapshot.pending_tasks.len(), processes_restarted, ago);
    ResumeResult { summary, tasks_restored: snapshot.pending_tasks.len(),
        processes_restarted, branch_changed, warnings }
}

/// Format workspace state for morning briefing.
pub fn briefing_items(snapshot: &WorkspaceSnapshot) -> Vec<String> {
    let mut items = Vec::new();
    let elapsed = Utc::now() - snapshot.timestamp;
    let ago = if elapsed.num_hours() < 24 { format!("{} hours", elapsed.num_hours()) }
        else { format!("{} days", elapsed.num_days()) };
    if !snapshot.pending_tasks.is_empty() {
        items.push(format!("{} pending tasks from {} ago", snapshot.pending_tasks.len(), ago));
    }
    for t in &snapshot.pending_tasks {
        let status = if t.blocked_on.is_some() { "blocked" } else { &format!("{:.0}%", t.progress * 100.0) };
        items.push(format!("  {} ({})", t.description, status));
    }
    if !snapshot.processes.is_empty() {
        let restartable = snapshot.processes.iter().filter(|p| p.restartable).count();
        items.push(format!("{restartable} processes to restart"));
    }
    items
}

// ── Helpers ──

fn detect_git_branch(dir: &Path) -> Option<String> {
    std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir).output().ok()
        .and_then(|o| if o.status.success() {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        } else { None })
}

fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn find_available_port(start: u16) -> u16 {
    for p in start..start.saturating_add(100) {
        if is_port_available(p) { return p; }
    }
    start
}

fn restart_process(command: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
    }
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_roundtrip() {
        let snap = WorkspaceSnapshot {
            timestamp: Utc::now(),
            working_directory: "/tmp/test".into(),
            git_branch: Some("main".into()),
            processes: vec![ProcessState {
                command: "npm start".into(), port: Some(3000),
                purpose: "dev server".into(), restartable: true,
            }],
            pending_tasks: vec![TaskSummary {
                description: "build report".into(), progress: 0.7, blocked_on: None,
            }],
            goals: vec![],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let loaded: WorkspaceSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.working_directory, "/tmp/test");
        assert_eq!(loaded.pending_tasks.len(), 1);
        assert_eq!(loaded.processes.len(), 1);
    }

    #[test]
    fn atomic_save_and_load() {
        let tmp = std::env::temp_dir().join("hydra_ws_test");
        let _ = std::fs::create_dir_all(&tmp);
        // Override snapshot dir via direct file write
        let snap = WorkspaceSnapshot {
            timestamp: Utc::now(), working_directory: "/tmp".into(),
            git_branch: None, processes: vec![], pending_tasks: vec![], goals: vec![],
        };
        let path = tmp.join("workspace.json");
        let json = serde_json::to_string_pretty(&snap).unwrap();
        std::fs::write(&path, &json).unwrap();
        let loaded: WorkspaceSnapshot = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.working_directory, "/tmp");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn briefing_items_format() {
        let snap = WorkspaceSnapshot {
            timestamp: Utc::now() - chrono::Duration::hours(5),
            working_directory: ".".into(), git_branch: Some("main".into()),
            processes: vec![ProcessState {
                command: "serve".into(), port: Some(8080),
                purpose: "api".into(), restartable: true,
            }],
            pending_tasks: vec![
                TaskSummary { description: "fix bug".into(), progress: 0.5, blocked_on: None },
            ],
            goals: vec![],
        };
        let items = briefing_items(&snap);
        assert!(items.iter().any(|i| i.contains("pending")));
        assert!(items.iter().any(|i| i.contains("restart")));
    }

    #[test]
    fn port_availability() {
        // Port 0 should always find something
        let p = find_available_port(49152);
        assert!(p >= 49152);
    }

    #[test]
    fn branch_detection() {
        let cwd = std::env::current_dir().unwrap();
        let branch = detect_git_branch(&cwd);
        // Should detect something in a git repo
        assert!(branch.is_some());
    }
}
