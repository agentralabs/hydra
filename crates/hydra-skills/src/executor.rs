//! SkillExecutor — sandboxed skill execution.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::definition::{SkillDefinition, SkillSource};
use crate::sandbox::Sandbox;
use crate::validator::SkillValidator;

/// Callback type for dispatching skill execution through a real bridge.
/// Arguments: (sister_id, tool_name, params) -> Result<Value, error_message>
pub type ToolDispatcher =
    Arc<dyn Fn(&str, &str, &serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("execution timed out after {0:?}")]
    Timeout(Duration),
    #[error("skill rejected by validator: {0}")]
    Rejected(String),
    #[error("missing required parameter: {0}")]
    MissingParam(String),
}

/// Result of executing a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub outputs: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    pub tokens_used: u64,
    pub error: Option<String>,
}

/// Executes skills with sandboxing and validation
pub struct SkillExecutor {
    validator: SkillValidator,
    default_timeout: Duration,
    tool_dispatcher: Option<ToolDispatcher>,
}

impl SkillExecutor {
    pub fn new() -> Self {
        Self {
            validator: SkillValidator::new(),
            default_timeout: Duration::from_secs(30),
            tool_dispatcher: None,
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            validator: SkillValidator::new(),
            default_timeout: timeout,
            tool_dispatcher: None,
        }
    }

    /// Set a real tool dispatcher for bridging skill execution through sister bridges.
    /// When set, MCP-sourced skills dispatch to the named sister; builtin skills
    /// dispatch with sister_id "builtin".
    pub fn with_dispatcher(mut self, dispatcher: ToolDispatcher) -> Self {
        self.tool_dispatcher = Some(dispatcher);
        self
    }

    /// Execute a skill with inputs
    pub fn execute(
        &self,
        skill: &SkillDefinition,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<SkillResult, ExecutionError> {
        let start = Instant::now();

        // 1. Validate the skill itself
        let validation = self.validator.validate(skill);
        if !validation.safe {
            return Err(ExecutionError::Rejected(validation.issues.join("; ")));
        }

        // 2. Validate inputs
        if let Err(errors) = skill.validate_inputs(&inputs) {
            return Err(ExecutionError::ValidationFailed(errors.join("; ")));
        }

        // 3. Create sandbox
        let sandbox = Sandbox::for_level(skill.sandbox_level);

        // 4. Check sandbox constraints
        if skill.needs_network() && !sandbox.allows_network() {
            return Err(ExecutionError::SandboxViolation(
                "skill requires network but sandbox denies it".into(),
            ));
        }

        // 5. Execute — dispatch through real bridge if available, otherwise simulate
        let elapsed = start.elapsed();
        if elapsed > self.default_timeout {
            return Err(ExecutionError::Timeout(self.default_timeout));
        }

        let outputs: HashMap<String, serde_json::Value> = if let Some(ref dispatcher) = self.tool_dispatcher {
            // Determine sister_id from skill source
            let sister_id = match &skill.source {
                SkillSource::Mcp { server } => server.as_str(),
                SkillSource::Builtin => "builtin",
                SkillSource::User => "user",
                SkillSource::OpenClaw => "openclaw",
            };

            // Derive tool name from first Tool trigger, or fall back to skill name
            let tool_name = skill
                .triggers
                .iter()
                .find_map(|t| {
                    if let crate::definition::SkillTrigger::Tool(name) = t {
                        Some(name.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or(&skill.name);

            // Build params value from inputs
            let params_value = serde_json::to_value(&inputs).unwrap_or(serde_json::Value::Null);

            // Dispatch through the real bridge
            let result = dispatcher(sister_id, tool_name, &params_value)
                .map_err(|e| ExecutionError::ValidationFailed(e))?;

            // Convert result into outputs map
            match result {
                serde_json::Value::Object(map) => map.into_iter().collect(),
                other => HashMap::from([("result".into(), other)]),
            }
        } else {
            // Fallback: simulated success with transformed inputs as outputs
            inputs
                .iter()
                .map(|(k, v)| (format!("result_{}", k), v.clone()))
                .collect()
        };

        Ok(SkillResult {
            success: true,
            outputs,
            duration_ms: start.elapsed().as_millis() as u64,
            tokens_used: 0,
            error: None,
        })
    }

    /// Check if a skill can be executed (dry run)
    pub fn can_execute(&self, skill: &SkillDefinition) -> bool {
        let validation = self.validator.validate(skill);
        validation.safe
    }
}

impl Default for SkillExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::*;

    fn test_skill(sandbox: SandboxLevel) -> SkillDefinition {
        SkillDefinition {
            id: "exec-1".into(),
            name: "test_exec".into(),
            version: "1.0.0".into(),
            description: "Test".into(),
            triggers: vec![SkillTrigger::Intent("test".into())],
            parameters: vec![SkillParam {
                name: "input".into(),
                param_type: ParamType::String,
                required: true,
                description: "Test input".into(),
                default: None,
                constraints: vec![],
            }],
            outputs: vec![],
            requirements: vec![],
            source: SkillSource::Builtin,
            sandbox_level: sandbox,
            risk_level: RiskLevel::Low,
            metadata: SkillMetadata::default(),
        }
    }

    #[test]
    fn test_executor_run() {
        let executor = SkillExecutor::new();
        let skill = test_skill(SandboxLevel::None);
        let inputs = HashMap::from([("input".into(), serde_json::json!("hello"))]);

        let result = executor.execute(&skill, inputs).unwrap();
        assert!(result.success);
        assert_eq!(result.tokens_used, 0);
        assert!(result.outputs.contains_key("result_input"));
    }

    #[test]
    fn test_executor_missing_param() {
        let executor = SkillExecutor::new();
        let skill = test_skill(SandboxLevel::None);
        let inputs = HashMap::new();

        let result = executor.execute(&skill, inputs);
        assert!(matches!(result, Err(ExecutionError::ValidationFailed(_))));
    }

    #[test]
    fn test_executor_sandbox_violation() {
        let executor = SkillExecutor::new();
        let mut skill = test_skill(SandboxLevel::Strict);
        skill.requirements.push(Requirement::Network);
        let inputs = HashMap::from([("input".into(), serde_json::json!("hi"))]);

        // Validator catches network+strict conflict before sandbox check
        let result = executor.execute(&skill, inputs);
        assert!(result.is_err());
    }

    #[test]
    fn test_executor_timeout_config() {
        let executor = SkillExecutor::with_timeout(Duration::from_secs(60));
        assert_eq!(executor.default_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_executor_default() {
        let executor = SkillExecutor::default();
        assert_eq!(executor.default_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_executor_can_execute_safe() {
        let executor = SkillExecutor::new();
        let skill = test_skill(SandboxLevel::None);
        assert!(executor.can_execute(&skill));
    }

    #[test]
    fn test_executor_can_execute_unsafe() {
        let executor = SkillExecutor::new();
        let mut skill = test_skill(SandboxLevel::Strict);
        skill.requirements.push(Requirement::Network);
        assert!(!executor.can_execute(&skill));
    }

    #[test]
    fn test_skill_result_serde() {
        let result = SkillResult {
            success: true,
            outputs: HashMap::from([("key".into(), serde_json::json!("val"))]),
            duration_ms: 42,
            tokens_used: 0,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: SkillResult = serde_json::from_str(&json).unwrap();
        assert!(restored.success);
        assert_eq!(restored.tokens_used, 0);
    }

    #[test]
    fn test_execution_error_display() {
        let err = ExecutionError::MissingParam("name".into());
        assert!(format!("{}", err).contains("name"));
        let err = ExecutionError::Timeout(Duration::from_secs(30));
        assert!(format!("{}", err).contains("30"));
    }

    #[test]
    fn test_executor_result_has_outputs() {
        let executor = SkillExecutor::new();
        let skill = test_skill(SandboxLevel::None);
        let inputs = HashMap::from([
            ("input".into(), serde_json::json!("test")),
        ]);
        let result = executor.execute(&skill, inputs).unwrap();
        assert!(result.outputs.contains_key("result_input"));
        assert_eq!(result.outputs["result_input"], serde_json::json!("test"));
    }
}
