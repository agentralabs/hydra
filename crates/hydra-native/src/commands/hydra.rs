//! Tauri-style commands — IPC bridge between frontend and hydra-runtime.
//!
//! These are plain async Rust functions that can be called from the Dioxus UI
//! or exposed as Tauri commands. The logic is testable without a UI.

use serde::{Deserialize, Serialize};

/// Result of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> CommandResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Status returned by get_status command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraStatus {
    pub connected: bool,
    pub version: String,
    pub current_run_id: Option<String>,
    pub pending_approval: bool,
    pub total_runs: u64,
}

/// Result of sending a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub run_id: String,
    pub status: String,
}

/// All commands the desktop can invoke against the hydra backend
pub struct HydraCommands {
    server_url: String,
    connected: parking_lot::Mutex<bool>,
    total_runs: parking_lot::Mutex<u64>,
}

impl HydraCommands {
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
            connected: parking_lot::Mutex::new(false),
            total_runs: parking_lot::Mutex::new(0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new("http://localhost:3000")
    }

    /// Send a message / intent to Hydra
    pub async fn send_message(&self, intent: &str) -> CommandResult<SendResult> {
        if intent.trim().is_empty() {
            return CommandResult::err("Intent cannot be empty");
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        *self.total_runs.lock() += 1;

        // In production: POST to /api/run with the intent
        // Response comes back via SSE events
        CommandResult::ok(SendResult {
            run_id,
            status: "started".into(),
        })
    }

    /// Kill/stop a running run
    pub async fn kill_run(&self, run_id: &str, level: &str) -> CommandResult<()> {
        if run_id.is_empty() {
            return CommandResult::err("run_id is required");
        }

        let valid_levels = ["graceful", "immediate", "halt"];
        if !valid_levels.contains(&level) {
            return CommandResult::err(format!(
                "Invalid kill level: {}. Use: {:?}",
                level, valid_levels
            ));
        }

        // In production: POST to /api/kill
        CommandResult::ok(())
    }

    /// Approve a pending decision
    pub async fn approve(&self, approval_id: &str, _approved: bool) -> CommandResult<()> {
        if approval_id.is_empty() {
            return CommandResult::err("approval_id is required");
        }

        // In production: POST to /api/approve
        CommandResult::ok(())
    }

    /// Get current Hydra status
    pub async fn get_status(&self) -> CommandResult<HydraStatus> {
        let connected = *self.connected.lock();
        let total_runs = *self.total_runs.lock();

        CommandResult::ok(HydraStatus {
            connected,
            version: "0.1.0".into(),
            current_run_id: None,
            pending_approval: false,
            total_runs,
        })
    }

    /// Start voice input
    pub async fn start_voice(&self) -> CommandResult<()> {
        // In production: activate microphone via platform APIs
        CommandResult::ok(())
    }

    /// Stop voice input
    pub async fn stop_voice(&self) -> CommandResult<()> {
        CommandResult::ok(())
    }

    /// Set connection state (called when SSE connects/disconnects)
    pub fn set_connected(&self, connected: bool) {
        *self.connected.lock() = connected;
    }

    /// Get the server URL
    pub fn server_url(&self) -> &str {
        &self.server_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        let cmds = HydraCommands::with_defaults();
        let result = cmds.send_message("Write a sort function").await;
        assert!(result.success);
        let data = result.data.unwrap();
        assert!(!data.run_id.is_empty());
        assert_eq!(data.status, "started");
    }

    #[tokio::test]
    async fn test_send_empty_message() {
        let cmds = HydraCommands::with_defaults();
        let result = cmds.send_message("").await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("empty"));
    }

    #[tokio::test]
    async fn test_kill_run() {
        let cmds = HydraCommands::with_defaults();
        let result = cmds.kill_run("run-123", "graceful").await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_kill_invalid_level() {
        let cmds = HydraCommands::with_defaults();
        let result = cmds.kill_run("run-123", "nuke").await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid kill level"));
    }

    #[tokio::test]
    async fn test_approve() {
        let cmds = HydraCommands::with_defaults();
        let result = cmds.approve("approval-1", true).await;
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_get_status() {
        let cmds = HydraCommands::with_defaults();
        cmds.set_connected(true);
        let result = cmds.get_status().await;
        assert!(result.success);
        let status = result.data.unwrap();
        assert!(status.connected);
        assert_eq!(status.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_voice_commands() {
        let cmds = HydraCommands::with_defaults();
        assert!(cmds.start_voice().await.success);
        assert!(cmds.stop_voice().await.success);
    }

    #[tokio::test]
    async fn test_run_counter() {
        let cmds = HydraCommands::with_defaults();
        cmds.send_message("task 1").await;
        cmds.send_message("task 2").await;
        let status = cmds.get_status().await.data.unwrap();
        assert_eq!(status.total_runs, 2);
    }
}
