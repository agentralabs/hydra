//! SkillExecutor — sandboxed skill execution.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::definition::SkillDefinition;
use crate::sandbox::Sandbox;
use crate::validator::SkillValidator;

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
}

impl SkillExecutor {
    pub fn new() -> Self {
        Self {
            validator: SkillValidator::new(),
            default_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            validator: SkillValidator::new(),
            default_timeout: timeout,
        }
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

        // 5. Execute (simulated — real execution would dispatch to runtime)
        let elapsed = start.elapsed();
        if elapsed > self.default_timeout {
            return Err(ExecutionError::Timeout(self.default_timeout));
        }

        // In production: dispatch to real runtime (MCP call, HTTP, subprocess)
        // For now: return simulated success with transformed inputs as outputs
        let outputs = inputs
            .iter()
            .map(|(k, v)| (format!("result_{}", k), v.clone()))
            .collect();

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
}
