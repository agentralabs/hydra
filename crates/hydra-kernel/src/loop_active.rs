//! The ACTIVE loop — processes commands from the principal.
//!
//! This is the foreground thread. It handles explicit instructions
//! from the human principal. Each command is constitutionally checked
//! before execution.

use crate::{
    errors::KernelError,
    state::HydraState,
    task_engine::{ManagedTask, TaskEngine},
};
use hydra_constitution::{
    ConstitutionChecker, LawCheckContext, constants::CONSTITUTIONAL_IDENTITY_ID,
};
use serde::{Deserialize, Serialize};

/// A command from the principal to the active loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActiveCommand {
    /// Execute a task described in natural language.
    Execute {
        /// The task description.
        description: String,
    },
    /// Query the current kernel state.
    QueryState,
    /// Shut down the kernel gracefully.
    Shutdown,
    /// Resume a suspended task.
    ResumeTask {
        /// The task ID to resume.
        task_id: String,
    },
}

/// The result of processing an active command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveResult {
    /// Whether the command was accepted.
    pub accepted: bool,
    /// Human-readable response.
    pub message: String,
    /// The task ID if one was created.
    pub task_id: Option<String>,
}

/// Process a single active command.
/// Returns an ActiveResult describing what happened.
pub async fn process_command(
    command: &ActiveCommand,
    state: &HydraState,
    engine: &mut TaskEngine,
) -> Result<ActiveResult, KernelError> {
    match command {
        ActiveCommand::Execute { description } => {
            // Constitutional check
            let checker = ConstitutionChecker::new();
            let ctx = LawCheckContext::new("active-cmd", "task.execute")
                .with_causal_chain(vec![CONSTITUTIONAL_IDENTITY_ID.to_string()]);

            let check = checker.check(&ctx);
            if !check.is_permitted() {
                return Ok(ActiveResult {
                    accepted: false,
                    message: format!(
                        "Constitutional violation: {}",
                        check
                            .first_violation()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    ),
                    task_id: None,
                });
            }

            let task = ManagedTask::new(description.clone());
            let id = engine.submit(task)?;

            Ok(ActiveResult {
                accepted: true,
                message: format!(
                    "Task submitted: '{}' (step={})",
                    description, state.step_count
                ),
                task_id: Some(id.as_str().to_string()),
            })
        }
        ActiveCommand::QueryState => Ok(ActiveResult {
            accepted: true,
            message: format!(
                "Kernel alive: step={}, V(Psi)={:.4}, tasks={}",
                state.step_count,
                state.lyapunov_value,
                engine.active_count()
            ),
            task_id: None,
        }),
        ActiveCommand::Shutdown => Ok(ActiveResult {
            accepted: true,
            message: "Shutdown initiated".to_string(),
            task_id: None,
        }),
        ActiveCommand::ResumeTask { task_id } => {
            let tid = hydra_constitution::TaskId::from_string(task_id.clone());
            match engine.get_mut(&tid) {
                Some(task) => {
                    task.resume_active();
                    Ok(ActiveResult {
                        accepted: true,
                        message: format!("Task {task_id} resumed"),
                        task_id: Some(task_id.clone()),
                    })
                }
                None => Ok(ActiveResult {
                    accepted: false,
                    message: format!("Task {task_id} not found"),
                    task_id: None,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn execute_command_creates_task() {
        let state = HydraState::initial();
        let mut engine = TaskEngine::new();
        let cmd = ActiveCommand::Execute {
            description: "build project".to_string(),
        };
        let result = process_command(&cmd, &state, &mut engine)
            .await
            .expect("should succeed");
        assert!(result.accepted);
        assert!(result.task_id.is_some());
        assert_eq!(engine.active_count(), 1);
    }

    #[tokio::test]
    async fn query_state_returns_info() {
        let state = HydraState::initial();
        let mut engine = TaskEngine::new();
        let cmd = ActiveCommand::QueryState;
        let result = process_command(&cmd, &state, &mut engine)
            .await
            .expect("should succeed");
        assert!(result.accepted);
        assert!(result.message.contains("step=0"));
    }

    #[tokio::test]
    async fn shutdown_accepted() {
        let state = HydraState::initial();
        let mut engine = TaskEngine::new();
        let cmd = ActiveCommand::Shutdown;
        let result = process_command(&cmd, &state, &mut engine)
            .await
            .expect("should succeed");
        assert!(result.accepted);
        assert!(result.message.contains("Shutdown"));
    }

    #[tokio::test]
    async fn resume_nonexistent_task_rejected() {
        let state = HydraState::initial();
        let mut engine = TaskEngine::new();
        let cmd = ActiveCommand::ResumeTask {
            task_id: "nonexistent".to_string(),
        };
        let result = process_command(&cmd, &state, &mut engine)
            .await
            .expect("should succeed");
        assert!(!result.accepted);
    }
}
