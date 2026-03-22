//! Task engine — manages the lifecycle of all in-flight tasks.
//!
//! Tasks never fail. They navigate obstacles.
//! The engine tracks every obstacle encounter, every approach tried,
//! and every reroute. Nothing is lost.

use crate::{constants::MAX_CONCURRENT_TASKS, errors::KernelError};
use chrono::{DateTime, Utc};
use hydra_constitution::declarations::HardStop;
use hydra_constitution::task::{
    ApproachType, AttemptOutcome, AttemptRecord, ObstacleType, TaskId, TaskState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A managed task within the kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedTask {
    /// The task's unique identifier.
    pub id: TaskId,

    /// Human-readable description of the task.
    pub description: String,

    /// Current state.
    pub state: TaskState,

    /// All approach attempts so far.
    pub attempts: Vec<AttemptRecord>,

    /// When the task was created.
    pub created_at: DateTime<Utc>,

    /// When the task last changed state.
    pub updated_at: DateTime<Utc>,

    /// The current approach cycle position.
    pub current_approach: ApproachType,

    /// How many times the approach cycle has completed.
    pub cycle_count: u32,
}

impl ManagedTask {
    /// Create a new task in the Active state.
    pub fn new(description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: TaskId::new(),
            description: description.into(),
            state: TaskState::Active,
            attempts: Vec::new(),
            created_at: now,
            updated_at: now,
            current_approach: ApproachType::first(),
            cycle_count: 0,
        }
    }

    /// Record an obstacle encounter and begin navigating it.
    pub fn hit_obstacle(&mut self, obstacle: ObstacleType) {
        self.state = TaskState::Blocked {
            obstacle,
            attempts: self.attempts.len(),
        };
        self.updated_at = Utc::now();
    }

    /// Try the next approach to navigate the current obstacle.
    pub fn reroute(&mut self, obstacle: ObstacleType) {
        let approach = self.current_approach.clone();
        let now = Utc::now();

        // Record the attempt
        self.attempts.push(AttemptRecord {
            approach: approach.clone(),
            obstacle,
            outcome: AttemptOutcome::Blocked {
                new_obstacle: "rerouting to next approach".to_string(),
            },
            started_at: self.updated_at.to_rfc3339(),
            ended_at: now.to_rfc3339(),
        });

        // Advance the approach cycle
        self.current_approach = approach.next();
        if self.current_approach == ApproachType::first() {
            self.cycle_count += 1;
        }

        self.state = TaskState::Rerouting {
            current_approach: self.current_approach.clone(),
        };
        self.updated_at = now;
    }

    /// Resume active execution after a reroute.
    pub fn resume_active(&mut self) {
        self.state = TaskState::Active;
        self.updated_at = Utc::now();
    }

    /// Mark the task as complete.
    pub fn complete(&mut self) {
        let now = Utc::now();
        self.attempts.push(AttemptRecord {
            approach: self.current_approach.clone(),
            obstacle: ObstacleType::Other {
                description: "none — task completed".to_string(),
            },
            outcome: AttemptOutcome::Succeeded,
            started_at: self.updated_at.to_rfc3339(),
            ended_at: now.to_rfc3339(),
        });
        self.state = TaskState::Complete;
        self.updated_at = now;
    }

    /// Apply a hard deny — the task cannot proceed.
    /// Requires explicit `HardStop` evidence from hydra-constitution.
    pub fn hard_deny(&mut self, evidence: HardStop) {
        let now = Utc::now();
        self.attempts.push(AttemptRecord {
            approach: self.current_approach.clone(),
            obstacle: ObstacleType::Other {
                description: evidence.description(),
            },
            outcome: AttemptOutcome::HardDenied {
                evidence: evidence.clone(),
            },
            started_at: self.updated_at.to_rfc3339(),
            ended_at: now.to_rfc3339(),
        });
        self.state = TaskState::HardDenied { evidence };
        self.updated_at = now;
    }

    /// Suspend the task waiting for external input.
    pub fn suspend(&mut self, waiting_for: String) {
        self.state = TaskState::Suspended { waiting_for };
        self.updated_at = Utc::now();
    }
}

/// The task engine — manages all in-flight tasks.
#[derive(Debug, Default)]
pub struct TaskEngine {
    /// All tasks, keyed by ID.
    tasks: HashMap<String, ManagedTask>,
}

impl TaskEngine {
    /// Create a new empty task engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Submit a new task. Returns an error if at capacity.
    pub fn submit(&mut self, task: ManagedTask) -> Result<TaskId, KernelError> {
        let active_count = self.tasks.values().filter(|t| t.state.is_active()).count();
        if active_count >= MAX_CONCURRENT_TASKS {
            return Err(KernelError::TaskEngineError {
                task_id: task.id.as_str().to_string(),
                reason: format!(
                    "At capacity: {active_count} active tasks (max {MAX_CONCURRENT_TASKS})"
                ),
            });
        }

        let id = task.id.clone();
        self.tasks.insert(id.as_str().to_string(), task);
        Ok(id)
    }

    /// Get a task by ID.
    pub fn get(&self, id: &TaskId) -> Option<&ManagedTask> {
        self.tasks.get(id.as_str())
    }

    /// Get a mutable task by ID.
    pub fn get_mut(&mut self, id: &TaskId) -> Option<&mut ManagedTask> {
        self.tasks.get_mut(id.as_str())
    }

    /// Record an obstacle on a task.
    pub fn record_obstacle(
        &mut self,
        id: &TaskId,
        obstacle: ObstacleType,
    ) -> Result<(), KernelError> {
        let task = self
            .tasks
            .get_mut(id.as_str())
            .ok_or_else(|| KernelError::TaskEngineError {
                task_id: id.as_str().to_string(),
                reason: "task not found".to_string(),
            })?;
        task.hit_obstacle(obstacle);
        Ok(())
    }

    /// Complete a task.
    pub fn complete_task(&mut self, id: &TaskId) -> Result<(), KernelError> {
        let task = self
            .tasks
            .get_mut(id.as_str())
            .ok_or_else(|| KernelError::TaskEngineError {
                task_id: id.as_str().to_string(),
                reason: "task not found".to_string(),
            })?;
        task.complete();
        Ok(())
    }

    /// Hard deny a task. Requires explicit HardStop evidence.
    pub fn hard_deny_task(&mut self, id: &TaskId, evidence: HardStop) -> Result<(), KernelError> {
        let task = self
            .tasks
            .get_mut(id.as_str())
            .ok_or_else(|| KernelError::TaskEngineError {
                task_id: id.as_str().to_string(),
                reason: "task not found".to_string(),
            })?;
        task.hard_deny(evidence);
        Ok(())
    }

    /// Number of active (non-terminal) tasks.
    pub fn active_count(&self) -> usize {
        self.tasks.values().filter(|t| t.state.is_active()).count()
    }

    /// Total number of tasks (including terminal).
    pub fn total_count(&self) -> usize {
        self.tasks.len()
    }

    /// All active task IDs.
    pub fn active_task_ids(&self) -> Vec<TaskId> {
        self.tasks
            .values()
            .filter(|t| t.state.is_active())
            .map(|t| t.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_is_active() {
        let task = ManagedTask::new("test task");
        assert!(task.state.is_active());
        assert_eq!(task.attempts.len(), 0);
    }

    #[test]
    fn obstacle_blocks_task() {
        let mut task = ManagedTask::new("test");
        task.hit_obstacle(ObstacleType::Timeout { duration_ms: 5000 });
        assert!(matches!(task.state, TaskState::Blocked { .. }));
    }

    #[test]
    fn reroute_advances_approach() {
        let mut task = ManagedTask::new("test");
        let initial = task.current_approach.clone();
        task.reroute(ObstacleType::Timeout { duration_ms: 100 });
        assert_ne!(
            format!("{:?}", task.current_approach),
            format!("{:?}", initial)
        );
        assert_eq!(task.attempts.len(), 1);
    }

    #[test]
    fn complete_sets_terminal() {
        let mut task = ManagedTask::new("test");
        task.complete();
        assert!(task.state.is_terminal());
        assert_eq!(task.state, TaskState::Complete);
    }

    #[test]
    fn hard_deny_sets_terminal() {
        let mut task = ManagedTask::new("test");
        task.hard_deny(HardStop::AuthenticationExplicitlyDenied {
            system: "server".to_string(),
            reason: "auth denied".to_string(),
            evidence: "Permission denied (publickey)".to_string(),
        });
        assert!(task.state.is_terminal());
    }

    #[test]
    fn engine_submit_and_retrieve() {
        let mut engine = TaskEngine::new();
        let task = ManagedTask::new("build project");
        let id = engine.submit(task).expect("submit should succeed");
        assert!(engine.get(&id).is_some());
        assert_eq!(engine.active_count(), 1);
    }

    #[test]
    fn engine_complete_task() {
        let mut engine = TaskEngine::new();
        let task = ManagedTask::new("test");
        let id = engine.submit(task).expect("submit");
        engine.complete_task(&id).expect("complete");
        assert_eq!(engine.active_count(), 0);
        let t = engine.get(&id).expect("should exist");
        assert_eq!(t.state, TaskState::Complete);
    }

    #[test]
    fn engine_hard_deny_task() {
        let mut engine = TaskEngine::new();
        let task = ManagedTask::new("test");
        let id = engine.submit(task).expect("submit");
        engine
            .hard_deny_task(
                &id,
                HardStop::PrincipalCancellation {
                    task_id: id.as_str().to_string(),
                    cancelled_at: "2026-03-19T12:00:00Z".to_string(),
                },
            )
            .expect("deny");
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn engine_record_obstacle() {
        let mut engine = TaskEngine::new();
        let task = ManagedTask::new("test");
        let id = engine.submit(task).expect("submit");
        engine
            .record_obstacle(&id, ObstacleType::Timeout { duration_ms: 100 })
            .expect("obstacle");
        let t = engine.get(&id).expect("should exist");
        assert!(matches!(t.state, TaskState::Blocked { .. }));
    }

    #[test]
    fn cycle_count_increments() {
        let mut task = ManagedTask::new("test");
        // Reroute 13 times to complete one cycle
        for _ in 0..13 {
            task.reroute(ObstacleType::Timeout { duration_ms: 100 });
        }
        assert_eq!(task.cycle_count, 1);
    }

    #[test]
    fn suspend_and_resume() {
        let mut task = ManagedTask::new("test");
        task.suspend("waiting for approval".to_string());
        assert!(matches!(task.state, TaskState::Suspended { .. }));
        assert!(task.state.is_active());

        task.resume_active();
        assert_eq!(task.state, TaskState::Active);
    }
}
