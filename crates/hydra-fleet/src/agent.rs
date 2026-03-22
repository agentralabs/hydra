//! Fleet agent — a single worker in the fleet.

use crate::constants::{AGENT_MAX_TASK_QUEUE, AGENT_NAME_MAX_LEN};
use crate::errors::FleetError;
use crate::task::FleetTask;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The operational state of a fleet agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FleetAgentState {
    /// Agent is idle and ready for work.
    Idle,
    /// Agent is actively working on a task.
    Working,
    /// Agent has a result ready for collection.
    ResultReady,
    /// Agent is quarantined due to failures.
    Quarantined,
    /// Agent is on constitutional hold.
    ConstitutionalHold,
    /// Agent has completed its lifecycle.
    Complete,
}

impl std::fmt::Display for FleetAgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Idle => "Idle",
            Self::Working => "Working",
            Self::ResultReady => "ResultReady",
            Self::Quarantined => "Quarantined",
            Self::ConstitutionalHold => "ConstitutionalHold",
            Self::Complete => "Complete",
        };
        write!(f, "{label}")
    }
}

/// What an agent specialises in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSpecialization {
    /// General-purpose agent.
    Generalist,
    /// Code analysis specialist.
    Analyst,
    /// Code generation specialist.
    Generator,
    /// Code review specialist.
    Reviewer,
    /// Security audit specialist.
    SecurityAuditor,
    /// Testing specialist.
    Tester,
    /// Documentation specialist.
    Documenter,
    /// Debugging specialist.
    Debugger,
}

impl std::fmt::Display for AgentSpecialization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Generalist => "Generalist",
            Self::Analyst => "Analyst",
            Self::Generator => "Generator",
            Self::Reviewer => "Reviewer",
            Self::SecurityAuditor => "SecurityAuditor",
            Self::Tester => "Tester",
            Self::Documenter => "Documenter",
            Self::Debugger => "Debugger",
        };
        write!(f, "{label}")
    }
}

/// A single agent in the fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetAgent {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable name (truncated to max length).
    pub name: String,
    /// Current operational state.
    pub state: FleetAgentState,
    /// What this agent specialises in.
    pub specialization: AgentSpecialization,
    /// Tasks assigned to this agent.
    pub task_queue: Vec<FleetTask>,
    /// Number of tasks completed successfully.
    pub tasks_completed: u64,
    /// Number of tasks that failed.
    pub tasks_failed: u64,
    /// When this agent was created.
    pub created_at: DateTime<Utc>,
    /// Last state transition time.
    pub last_transition: DateTime<Utc>,
}

impl FleetAgent {
    /// Create a new fleet agent with a given name and specialization.
    pub fn new(name: impl Into<String>, specialization: AgentSpecialization) -> Self {
        let now = Utc::now();
        let mut agent_name = name.into();
        agent_name.truncate(AGENT_NAME_MAX_LEN);
        Self {
            id: Uuid::new_v4(),
            name: agent_name,
            state: FleetAgentState::Idle,
            specialization,
            task_queue: Vec::new(),
            tasks_completed: 0,
            tasks_failed: 0,
            created_at: now,
            last_transition: now,
        }
    }

    /// Assign a task to this agent.
    pub fn assign_task(&mut self, task: FleetTask) -> Result<(), FleetError> {
        if self.state == FleetAgentState::Quarantined {
            return Err(FleetError::AgentQuarantined {
                agent_id: self.id.to_string(),
            });
        }
        if self.task_queue.len() >= AGENT_MAX_TASK_QUEUE {
            return Err(FleetError::TaskQueueFull {
                agent_id: self.id.to_string(),
                current: self.task_queue.len(),
                max: AGENT_MAX_TASK_QUEUE,
            });
        }
        self.task_queue.push(task);
        self.state = FleetAgentState::Working;
        self.last_transition = Utc::now();
        Ok(())
    }

    /// Mark the agent as having a result ready.
    pub fn result_ready(&mut self) {
        self.state = FleetAgentState::ResultReady;
        self.last_transition = Utc::now();
    }

    /// Complete the current task successfully.
    pub fn complete_task(&mut self) {
        self.tasks_completed += 1;
        if !self.task_queue.is_empty() {
            self.task_queue.remove(0);
        }
        if self.task_queue.is_empty() {
            self.state = FleetAgentState::Idle;
        } else {
            self.state = FleetAgentState::Working;
        }
        self.last_transition = Utc::now();
    }

    /// Quarantine this agent.
    pub fn quarantine(&mut self) {
        self.state = FleetAgentState::Quarantined;
        self.last_transition = Utc::now();
    }

    /// Return the success rate as a fraction in [0.0, 1.0].
    /// Returns 1.0 if no tasks have been attempted.
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 {
            return 1.0;
        }
        self.tasks_completed as f64 / total as f64
    }
}
