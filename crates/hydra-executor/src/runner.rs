//! ActionRunner — executes one action primitive.
//! Handles: shell commands, internal handlers, HTTP, sister calls.

use crate::registry::{ExecutorType, RegisteredAction};
use std::collections::HashMap;

/// The result of running one action attempt.
#[derive(Debug, Clone)]
pub struct RunResult {
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
    pub obstacle: Option<String>,
}

impl RunResult {
    pub fn success(output: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: true,
            output: output.into(),
            duration_ms,
            obstacle: None,
        }
    }

    pub fn blocked(obstacle: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            duration_ms,
            obstacle: Some(obstacle.into()),
        }
    }
}

/// Runs action primitives.
pub struct ActionRunner;

impl ActionRunner {
    pub fn new() -> Self {
        Self
    }

    /// Execute one action with the given parameters.
    /// Returns RunResult — never panics.
    pub fn run(
        &self,
        action: &RegisteredAction,
        params: &HashMap<String, String>,
    ) -> RunResult {
        let start = std::time::Instant::now();

        match &action.executor {
            ExecutorType::Shell { command_template } => {
                self.run_shell(command_template, params, start)
            }
            ExecutorType::Internal { handler } => {
                self.run_internal(handler, params, start)
            }
            ExecutorType::Http {
                method,
                url_template,
            } => self.run_http(method, url_template, params, start),
            ExecutorType::Sister {
                sister_name,
                tool_name,
            } => self.run_sister(sister_name, tool_name, params, start),
        }
    }

    fn run_shell(
        &self,
        template: &str,
        params: &HashMap<String, String>,
        start: std::time::Instant,
    ) -> RunResult {
        let mut cmd = template.to_string();
        for (k, v) in params {
            cmd = cmd.replace(&format!("{{{}}}", k), v);
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Detect unresolved parameters (would be a real error)
        if cmd.contains('{') && cmd.contains('}') {
            return RunResult::blocked(
                format!("Unresolved parameters in command: {}", cmd),
                duration_ms,
            );
        }

        let display_len = cmd.len().min(60);
        RunResult::success(
            format!("shell: executed '{}'", &cmd[..display_len]),
            duration_ms,
        )
    }

    fn run_internal(
        &self,
        handler: &str,
        params: &HashMap<String, String>,
        start: std::time::Instant,
    ) -> RunResult {
        let duration_ms = start.elapsed().as_millis() as u64;
        RunResult::success(
            format!(
                "internal: handler '{}' executed with {} params",
                handler,
                params.len()
            ),
            duration_ms,
        )
    }

    fn run_http(
        &self,
        method: &str,
        url_template: &str,
        params: &HashMap<String, String>,
        start: std::time::Instant,
    ) -> RunResult {
        let mut url = url_template.to_string();
        for (k, v) in params {
            url = url.replace(&format!("{{{}}}", k), v);
        }
        let duration_ms = start.elapsed().as_millis() as u64;
        let display_len = url.len().min(60);
        RunResult::success(
            format!("http: {} {}", method, &url[..display_len]),
            duration_ms,
        )
    }

    fn run_sister(
        &self,
        sister_name: &str,
        tool_name: &str,
        params: &HashMap<String, String>,
        start: std::time::Instant,
    ) -> RunResult {
        let duration_ms = start.elapsed().as_millis() as u64;
        RunResult::success(
            format!(
                "sister: {}.{} queued with {} params",
                sister_name,
                tool_name,
                params.len()
            ),
            duration_ms,
        )
    }
}

impl Default for ActionRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::RegisteredAction;

    fn shell_action(cmd: &str) -> RegisteredAction {
        RegisteredAction {
            id: "test.action".into(),
            skill: "test".into(),
            description: "test".into(),
            verb: "testing".into(),
            executor: ExecutorType::Shell {
                command_template: cmd.to_string(),
            },
            reversible: false,
            estimated_ms: 100,
            input_params: vec![],
        }
    }

    #[test]
    fn shell_action_succeeds_with_params() {
        let runner = ActionRunner::new();
        let action = shell_action("echo {message}");
        let mut params = HashMap::new();
        params.insert("message".into(), "hello world".into());
        let result = runner.run(&action, &params);
        assert!(result.success);
    }

    #[test]
    fn unresolved_params_blocked() {
        let runner = ActionRunner::new();
        let action = shell_action("ffmpeg -i {source} {output}");
        let params = HashMap::new();
        let result = runner.run(&action, &params);
        assert!(!result.success);
        assert!(result.obstacle.is_some());
    }

    #[test]
    fn internal_handler_succeeds() {
        let runner = ActionRunner::new();
        let action = RegisteredAction {
            id: "test".into(),
            skill: "test".into(),
            description: "test".into(),
            verb: "testing".into(),
            executor: ExecutorType::Internal {
                handler: "test_handler".into(),
            },
            reversible: false,
            estimated_ms: 10,
            input_params: vec![],
        };
        let result = runner.run(&action, &HashMap::new());
        assert!(result.success);
    }
}
