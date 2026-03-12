//! Sister Dispatch Matrix — routes cognitive phases to the 14 sisters.
//! ACT and LEARN implementations are in dispatch_act.rs.
//! PERCEIVE, THINK, DECIDE, and ASSESS_RISK implementations are in dispatch_phases.rs.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tracing::{debug, warn};

use hydra_core::error::HydraError;
use hydra_core::types::RiskAssessment;
use hydra_gate::ExecutionGate;
use hydra_intent::IntentCompiler;
use hydra_sisters::bridge::{SisterAction, SisterId};
use hydra_sisters::SisterRegistry;

use crate::cognitive_loop::{CycleInput, PhaseHandler};

/// Context passed through the cognitive pipeline
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DispatchContext {
    /// Raw user input
    pub input_text: String,
    /// Compiled intent (set after THINK)
    pub intent: Option<serde_json::Value>,
    /// Perceived context from sisters
    pub perceived: serde_json::Value,
    /// Plan from THINK phase
    pub plan: serde_json::Value,
    /// Gate decision from DECIDE phase
    pub gate_decision: Option<String>,
    /// Whether code is involved
    pub involves_code: bool,
    /// Whether visual reference is involved
    pub involves_vision: bool,
    /// Whether network/communication is needed
    pub involves_network: bool,
    /// Risk level assessed
    pub risk_level: String,
}

impl Default for DispatchContext {
    fn default() -> Self {
        Self {
            input_text: String::new(),
            intent: None,
            perceived: json!({}),
            plan: json!({}),
            gate_decision: None,
            involves_code: false,
            involves_vision: false,
            involves_network: false,
            risk_level: "none".into(),
        }
    }
}

/// The Sister Dispatcher — implements PhaseHandler by routing to 14 sisters
pub struct SisterDispatcher {
    registry: Arc<SisterRegistry>,
    pub(crate) compiler: Arc<IntentCompiler>,
    _gate: Arc<ExecutionGate>,
    sisters_online: bool,
}

impl SisterDispatcher {
    pub fn new(
        registry: Arc<SisterRegistry>,
        compiler: Arc<IntentCompiler>,
        gate: Arc<ExecutionGate>,
    ) -> Self {
        Self {
            registry,
            compiler,
            _gate: gate,
            sisters_online: true,
        }
    }

    /// Call a sister, returning its result or a graceful fallback
    pub(crate) async fn call_sister(
        &self,
        sister_id: SisterId,
        tool: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        match self.registry.get(sister_id) {
            Some(bridge) => {
                let action = SisterAction::new(tool, params);
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    bridge.call(action),
                )
                .await
                {
                    Ok(Ok(result)) => result.data,
                    Ok(Err(e)) => {
                        warn!("Sister {:?} error: {}", sister_id, e);
                        json!({"error": e.to_string(), "sister": sister_id.name()})
                    }
                    Err(_) => {
                        warn!("Sister {:?} timed out", sister_id);
                        json!({"timeout": true, "sister": sister_id.name()})
                    }
                }
            }
            None => {
                debug!("Sister {:?} not registered", sister_id);
                json!({"unavailable": true, "sister": sister_id.name()})
            }
        }
    }

    /// Check if the input involves code operations.
    /// Prefers intent category from CycleInput context (set by micro-LLM classifier),
    /// falls back to keyword heuristic when no intent is available.
    pub(crate) fn involves_code(input: &CycleInput) -> bool {
        // Check intent category from micro-LLM classifier first
        if let Some(cat) = input.context.get("intent_category").and_then(|c| c.as_str()) {
            return matches!(cat, "code_build" | "code_fix" | "code_explain" | "self_repair" | "self_scan");
        }
        // Fallback: keyword heuristic (only when no intent classification available)
        let code_indicators = [
            ".rs", ".ts", ".py", ".js", ".go", ".java", "src/", "crates/",
            "cargo ", "npm ", "compile", "build",
        ];
        let lower = input.text.to_lowercase();
        code_indicators.iter().any(|kw| lower.contains(kw))
    }

    /// Check if the input involves visual content.
    /// Prefers intent category from CycleInput context, falls back to keyword heuristic.
    pub(crate) fn involves_vision(input: &CycleInput) -> bool {
        if let Some(cat) = input.context.get("intent_category").and_then(|c| c.as_str()) {
            return matches!(cat, "web_browse"); // Vision used for web browsing
        }
        let vision_indicators = [
            "screenshot", "image", "photo", ".png", ".jpg", ".svg", ".gif",
        ];
        let lower = input.text.to_lowercase();
        vision_indicators.iter().any(|kw| lower.contains(kw))
    }

    /// Check if the input involves network/communication.
    /// Prefers intent category from CycleInput context, falls back to keyword heuristic.
    pub(crate) fn involves_network(input: &CycleInput) -> bool {
        if let Some(cat) = input.context.get("intent_category").and_then(|c| c.as_str()) {
            return matches!(cat, "communicate" | "deploy");
        }
        let net_indicators = [
            "send ", "email", "slack", "webhook", "http://", "https://",
        ];
        let lower = input.text.to_lowercase();
        net_indicators.iter().any(|kw| lower.contains(kw))
    }
}

#[async_trait]
impl PhaseHandler for SisterDispatcher {
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        self.perceive_impl(input).await
    }

    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.think_impl(perceived).await
    }

    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.decide_impl(thought).await
    }

    async fn assess_risk(
        &self,
        decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        self.assess_risk_impl(decision).await
    }

    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.act_impl(decision).await
    }

    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        self.learn_impl(result).await
    }

    fn sisters_available(&self) -> bool {
        self.sisters_online
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "dispatch_tests_extra.rs"]
mod tests_extra;
