use std::time::Instant;

use async_trait::async_trait;
use tracing::{info, warn};

use hydra_core::error::HydraError;
use hydra_core::types::{CognitivePhase, RiskAssessment, RiskFactor, RiskLevel};
use hydra_kernel::cognitive_loop::{CycleInput, PhaseHandler};
use hydra_model::{LlmConfig, ModelExecutor, ModelRegistry};

use super::prompts;
use super::types::*;

/// Configuration for the LLM-backed cognitive loop
#[derive(Debug, Clone)]
pub struct CognitiveLoopConfig {
    pub perception_model: String,
    pub thinking_model: String,
    pub decision_model: String,
}

impl Default for CognitiveLoopConfig {
    fn default() -> Self {
        Self {
            perception_model: "claude-haiku".into(),
            thinking_model: "claude-sonnet".into(),
            decision_model: "claude-haiku".into(),
        }
    }
}

/// LLM-backed phase handler that calls real models via ModelExecutor
pub struct LlmPhaseHandler {
    executor: ModelExecutor,
    config: CognitiveLoopConfig,
    phase_tokens: parking_lot::Mutex<Vec<(CognitivePhase, u64, u64)>>,
}

impl LlmPhaseHandler {
    pub fn new(executor: ModelExecutor, config: CognitiveLoopConfig) -> Self {
        Self {
            executor,
            config,
            phase_tokens: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn with_defaults() -> Self {
        let registry = ModelRegistry::new();
        let executor = ModelExecutor::new(registry);
        Self::new(executor, CognitiveLoopConfig::default())
    }

    pub fn with_llm_config(llm_config: LlmConfig) -> Self {
        let registry = ModelRegistry::new();
        let executor = ModelExecutor::with_config(registry, llm_config);
        Self::new(executor, CognitiveLoopConfig::default())
    }

    /// Get token usage per phase
    pub fn phase_metrics(&self) -> Vec<(CognitivePhase, u64, u64)> {
        self.phase_tokens.lock().clone()
    }

    /// Total tokens used across all phases
    pub fn total_tokens(&self) -> u64 {
        self.phase_tokens.lock().iter().map(|(_, t, _)| *t).sum()
    }

    fn record_phase(&self, phase: CognitivePhase, tokens: u64, duration_ms: u64) {
        self.phase_tokens.lock().push((phase, tokens, duration_ms));
    }

    /// Execute a model call and parse the JSON response with fallback
    async fn call_model_json<T: serde::de::DeserializeOwned + Default>(
        &self,
        model_id: &str,
        _system: &str,
        user_message: &str,
        phase: CognitivePhase,
    ) -> Result<(T, u64), HydraError> {
        let start = Instant::now();

        let result = self
            .executor
            .execute(model_id, user_message, &[])
            .await
            .map_err(|e| HydraError::Internal(format!("LLM call failed: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let tokens = result.tokens_used;

        // Extract the response text
        let response_text = result
            .output
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Try to parse JSON from the response
        let parsed = parse_json_with_fallback::<T>(response_text);

        self.record_phase(phase, tokens, duration_ms);
        info!(
            phase = ?phase,
            tokens = tokens,
            duration_ms = duration_ms,
            "Cognitive phase completed"
        );

        Ok((parsed, tokens))
    }
}

#[async_trait]
impl PhaseHandler for LlmPhaseHandler {
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        let user_msg = format!(
            "User input: {}\n\nAdditional context: {}",
            input.text, input.context
        );

        let (perception, _tokens): (Perception, _) = self
            .call_model_json(
                &self.config.perception_model,
                prompts::perceive_system_prompt(),
                &user_msg,
                CognitivePhase::Perceive,
            )
            .await?;

        // If the LLM returned defaults (mock fallback), fill in from input
        let perception = if perception.intent.is_empty() {
            Perception {
                intent: input.text.clone(),
                intent_type: "general".into(),
                ..perception
            }
        } else {
            perception
        };

        serde_json::to_value(&perception)
            .map_err(|e| HydraError::Internal(format!("Serialize perception: {}", e)))
    }

    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        let user_msg = format!("Perception: {}", perceived);

        let (thinking, _tokens): (ThinkingResult, _) = self
            .call_model_json(
                &self.config.thinking_model,
                prompts::think_system_prompt(),
                &user_msg,
                CognitivePhase::Think,
            )
            .await?;

        serde_json::to_value(&thinking)
            .map_err(|e| HydraError::Internal(format!("Serialize thinking: {}", e)))
    }

    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        let user_msg = format!("Thinking: {}", thought);

        let (decision, _tokens): (Decision, _) = self
            .call_model_json(
                &self.config.decision_model,
                prompts::decide_system_prompt(),
                &user_msg,
                CognitivePhase::Decide,
            )
            .await?;

        serde_json::to_value(&decision)
            .map_err(|e| HydraError::Internal(format!("Serialize decision: {}", e)))
    }

    async fn assess_risk(
        &self,
        decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        // Risk assessment is rule-based, not LLM-based
        let action = decision
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let reversible = decision
            .get("reversible")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let level = if !reversible {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(RiskAssessment {
            level,
            factors: vec![RiskFactor {
                name: "action_analysis".into(),
                severity: level,
                description: format!("Action: {}, reversible: {}", action, reversible),
            }],
            mitigations: vec![],
            requires_approval: level >= RiskLevel::High,
        })
    }

    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        let start = Instant::now();

        // ACT phase executes via sisters or LLM depending on the action
        // For now, record the decision as the action result
        let action = decision
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("none");

        let duration_ms = start.elapsed().as_millis() as u64;
        self.record_phase(CognitivePhase::Act, 0, duration_ms);

        Ok(serde_json::json!({
            "executed": true,
            "action": action,
            "duration_ms": duration_ms,
        }))
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        let start = Instant::now();

        // LEARN phase: summarize and decide what to remember
        // Uses sister calls (Memory, Cognition) rather than LLM tokens
        let duration_ms = start.elapsed().as_millis() as u64;
        self.record_phase(CognitivePhase::Learn, 0, duration_ms);

        Ok(serde_json::json!({
            "learned": true,
            "summary": format!("Completed action: {:?}", result.get("action")),
            "should_remember": true,
        }))
    }
}

/// Parse JSON from LLM response, with fallback to defaults on failure.
/// Handles cases where the LLM wraps JSON in markdown code blocks.
pub fn parse_json_with_fallback<T: serde::de::DeserializeOwned + Default>(text: &str) -> T {
    // Try direct parse
    if let Ok(parsed) = serde_json::from_str::<T>(text) {
        return parsed;
    }

    // Try extracting JSON from markdown code block
    let stripped = text.trim();
    if let Some(start) = stripped.find('{') {
        if let Some(end) = stripped.rfind('}') {
            let json_str = &stripped[start..=end];
            if let Ok(parsed) = serde_json::from_str::<T>(json_str) {
                return parsed;
            }
        }
    }

    warn!(
        "Failed to parse LLM JSON response, using defaults. Text: {:.100}",
        text
    );
    T::default()
}
