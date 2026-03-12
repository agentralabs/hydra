//! Agent types — instance, config, status, role, permissions, task, result.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique agent identifier.
pub type AgentId = String;

/// Where an agent runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentHost {
    Local,
    Remote { host: String, user: String },
}

/// What kind of agent this is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    Generalist,
    Specialist(String),
    Worker,
    Monitor,
}

/// Current status of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Starting,
    Idle,
    Working(String),
    Completed,
    Failed(String),
    Terminated,
}

/// Permissions granted to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissions {
    pub can_write_files: bool,
    pub can_execute_commands: bool,
    pub can_access_network: bool,
    pub can_spawn_subagents: bool,
    pub max_cost_cents: u64,
    pub allowed_directories: Vec<String>,
    pub blocked_commands: Vec<String>,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            can_write_files: true,
            can_execute_commands: true,
            can_access_network: false,
            can_spawn_subagents: false,
            max_cost_cents: 100,
            allowed_directories: vec![],
            blocked_commands: vec![
                "rm -rf /".into(),
                "dd if=/dev/zero".into(),
                "shutdown".into(),
            ],
        }
    }
}

/// Configuration for spawning an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub host: AgentHost,
    pub skills: Vec<String>,
    pub permissions: AgentPermissions,
    pub goal: Option<String>,
}

/// A task assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub required_skills: Vec<String>,
    pub priority: u8,
    pub timeout_secs: u64,
}

/// Result from a completed agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub agent_id: String,
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub quality_score: f64,
    pub completed_at: DateTime<Utc>,
}

/// A single agent instance managed by the swarm.
#[derive(Debug, Clone)]
pub struct AgentInstance {
    pub id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub status: AgentStatus,
    pub host: AgentHost,
    pub skills: Vec<String>,
    pub permissions: AgentPermissions,
    pub task: Option<AgentTask>,
    pub started_at: DateTime<Utc>,
    pub results: Vec<TaskResult>,
    /// PID for local agents, None for remote.
    pub pid: Option<u32>,
}

impl AgentInstance {
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: config.name.clone(),
            role: config.role.clone(),
            status: AgentStatus::Starting,
            host: config.host.clone(),
            skills: config.skills.clone(),
            permissions: config.permissions.clone(),
            task: None,
            started_at: Utc::now(),
            results: Vec::new(),
            pid: None,
        }
    }

    /// Check if agent can handle a task based on skills.
    pub fn can_handle(&self, task: &AgentTask) -> bool {
        if self.status != AgentStatus::Idle {
            return false;
        }
        if task.required_skills.is_empty() {
            return true;
        }
        task.required_skills.iter()
            .all(|req| self.skills.iter().any(|s| s == req))
    }

    /// Assign a task to this agent.
    pub fn assign_task(&mut self, task: AgentTask) {
        self.status = AgentStatus::Working(task.description.clone());
        self.task = Some(task);
    }

    /// Mark task complete with result.
    pub fn complete_task(&mut self, result: TaskResult) {
        self.status = AgentStatus::Completed;
        self.task = None;
        self.results.push(result);
    }

    /// Mark agent as failed.
    pub fn mark_failed(&mut self, error: &str) {
        self.status = AgentStatus::Failed(error.to_string());
        self.task = None;
    }

    /// Mark agent as idle (ready for work).
    pub fn mark_idle(&mut self) {
        self.status = AgentStatus::Idle;
    }

    /// Display summary for dashboard.
    pub fn summary(&self) -> String {
        let host = match &self.host {
            AgentHost::Local => "local".to_string(),
            AgentHost::Remote { host, .. } => host.clone(),
        };
        let status = match &self.status {
            AgentStatus::Starting => "starting".into(),
            AgentStatus::Idle => "idle".into(),
            AgentStatus::Working(desc) => format!("working: {}", desc),
            AgentStatus::Completed => "completed".into(),
            AgentStatus::Failed(e) => format!("failed: {}", e),
            AgentStatus::Terminated => "terminated".into(),
        };
        format!("[{}] {} ({}) on {} — {}",
            &self.id[..8], self.name, format!("{:?}", self.role), host, status)
    }
}

/// Task assignment mapping.
#[derive(Debug, Clone)]
pub struct Assignment {
    pub agent_id: AgentId,
    pub task: AgentTask,
}

/// Health status of an agent.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub agent_id: AgentId,
    pub alive: bool,
    pub responsive: bool,
    pub error: Option<String>,
}
