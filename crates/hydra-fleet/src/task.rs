//! Task definitions for fleet agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The type of work a task represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// Analyse source code or configuration.
    CodeAnalysis,
    /// Generate new code from a specification.
    CodeGeneration,
    /// Review code for quality and correctness.
    CodeReview,
    /// Perform a security audit.
    SecurityAudit,
    /// Run tests and report results.
    Testing,
    /// Generate or update documentation.
    Documentation,
    /// Refactor existing code.
    Refactoring,
    /// Debug and diagnose an issue.
    Debugging,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::CodeAnalysis => "CodeAnalysis",
            Self::CodeGeneration => "CodeGeneration",
            Self::CodeReview => "CodeReview",
            Self::SecurityAudit => "SecurityAudit",
            Self::Testing => "Testing",
            Self::Documentation => "Documentation",
            Self::Refactoring => "Refactoring",
            Self::Debugging => "Debugging",
        };
        write!(f, "{label}")
    }
}

/// Maximum priority value for tasks.
const MAX_PRIORITY: u8 = 10;

/// A unit of work assigned to a fleet agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetTask {
    /// Unique identifier for this task.
    pub id: Uuid,
    /// What kind of work this task represents.
    pub task_type: TaskType,
    /// Human-readable description of the task.
    pub description: String,
    /// Priority level (0 = lowest, clamped at 10).
    pub priority: u8,
    /// When this task was created.
    pub created_at: DateTime<Utc>,
    /// The causal root that initiated this task chain.
    pub causal_root: String,
}

impl FleetTask {
    /// Create a new task with priority clamped to the maximum.
    pub fn new(
        task_type: TaskType,
        description: impl Into<String>,
        priority: u8,
        causal_root: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            description: description.into(),
            priority: priority.min(MAX_PRIORITY),
            created_at: Utc::now(),
            causal_root: causal_root.into(),
        }
    }
}
