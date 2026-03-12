//! PERCEIVE + THINK + DECIDE phase dispatch — extracted from dispatch.rs for file size.
//! Contains the perceive(), think(), and decide() PhaseHandler implementations.

use serde_json::json;
use tracing::info;

use hydra_core::error::HydraError;
use hydra_core::types::{RiskAssessment, RiskLevel};
use hydra_sisters::bridge::SisterId;

use super::dispatch::SisterDispatcher;
use crate::cognitive_loop::CycleInput;

impl SisterDispatcher {
    /// PERCEIVE: Gather context from relevant sisters (10s timeout)
    ///
    /// Always called: Memory, Time, Cognition, Reality
    /// Conditionally: Vision (if visual), Codebase (if code)
    pub(crate) async fn perceive_impl(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
        info!(phase = "perceive", "Gathering context from sisters");

        let text = &input.text;
        let code = Self::involves_code(input);
        let vision = Self::involves_vision(input);

        // Always-call sisters (parallel) — V3/V4 memory search alongside V2
        let (memory_ctx, longevity_ctx, time_ctx, cognition_ctx, reality_ctx) = tokio::join!(
            // V2: Graph-based memory query (fast, local)
            self.call_sister(
                SisterId::Memory,
                "memory_query",
                json!({"query": text, "limit": 5})
            ),
            // V4: Longevity search across 20-year hierarchy (semantic, deep)
            self.call_sister(
                SisterId::Memory,
                "memory_longevity_search",
                json!({"query": text, "limit": 5, "include_layers": ["episode", "summary", "pattern"]})
            ),
            self.call_sister(
                SisterId::Time,
                "time_stats",
                json!({})
            ),
            self.call_sister(
                SisterId::Cognition,
                "cognition_model_query",
                json!({"context": "current_user"})
            ),
            self.call_sister(
                SisterId::Reality,
                "reality_context",
                json!({"input": text})
            ),
        );

        // Conditional sisters (parallel where applicable)
        let codebase_ctx = if code {
            self.call_sister(
                SisterId::Codebase,
                "search_semantic",
                json!({"query": text}),
            )
            .await
        } else {
            json!(null)
        };

        let vision_ctx = if vision {
            self.call_sister(
                SisterId::Vision,
                "vision_capture",
                json!({"context": text}),
            )
            .await
        } else {
            json!(null)
        };

        let context = json!({
            "input": text,
            "involves_code": code,
            "involves_vision": vision,
            "involves_network": Self::involves_network(input),
            "memory": memory_ctx,
            "longevity": longevity_ctx,
            "temporal": time_ctx,
            "user_model": cognition_ctx,
            "reality": reality_ctx,
            "codebase": codebase_ctx,
            "vision": vision_ctx,
        });

        Ok(context)
    }

    /// THINK: Compile intent, decompose goals, generate plans (60s timeout)
    ///
    /// Always called: Veritas (intent compilation), Planning (decomposition), Cognition (belief check)
    /// Conditionally: Forge (if code generation), Memory (if context needed)
    pub(crate) async fn think_impl(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        info!(phase = "think", "Compiling intent and planning");

        let input_text = perceived["input"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let involves_code = perceived["involves_code"].as_bool().unwrap_or(false);

        // Stage 1: Compile intent via local compiler (0 tokens if cached/classified)
        let mut budget = hydra_core::types::TokenBudget::new(5000);
        let compile_result = self.compiler.compile(&input_text, &mut budget).await;

        // Stage 2: Verify intent via Veritas sister
        let veritas_result = self
            .call_sister(
                SisterId::Veritas,
                "veritas_compile_intent",
                json!({
                    "text": input_text,
                    "compiled": compile_result.is_ok(),
                    "confidence": compile_result.intent.as_ref().map(|i| i.confidence).unwrap_or(0.0),
                }),
            )
            .await;

        // Stage 3: Plan decomposition via Planning sister
        let planning_result = self
            .call_sister(
                SisterId::Planning,
                "planning_goal",
                json!({
                    "operation": "decompose",
                    "text": input_text,
                    "context": perceived,
                }),
            )
            .await;

        // Stage 4: Check beliefs via Cognition
        let belief_check = self
            .call_sister(
                SisterId::Cognition,
                "cognition_belief_query",
                json!({"context": input_text}),
            )
            .await;

        // Stage 5: Generate blueprint if code involved
        let forge_result = if involves_code {
            self.call_sister(
                SisterId::Forge,
                "forge_blueprint_create",
                json!({
                    "description": input_text,
                    "context": perceived.get("codebase"),
                }),
            )
            .await
        } else {
            json!(null)
        };

        let intent_json = compile_result
            .intent
            .as_ref()
            .map(|i| json!({
                "id": i.id.to_string(),
                "goal_type": format!("{:?}", i.goal.goal_type),
                "actions": i.actions.iter().map(|a| format!("{:?}", a.action_type)).collect::<Vec<_>>(),
                "confidence": i.confidence,
                "tokens_used": compile_result.tokens_used,
                "status": format!("{:?}", compile_result.status),
            }))
            .unwrap_or(json!(null));

        Ok(json!({
            "input": input_text,
            "involves_code": involves_code,
            "involves_vision": perceived["involves_vision"],
            "involves_network": perceived["involves_network"],
            "intent": intent_json,
            "veritas": veritas_result,
            "plan": planning_result,
            "beliefs": belief_check,
            "blueprint": forge_result,
            "perceived": perceived,
        }))
    }

    /// DECIDE: Gate check, risk assessment, approval routing (30s timeout)
    ///
    /// Always called: Contract (policy check), Aegis (safety validation)
    /// Conditionally: Identity (prepare action token if approved)
    pub(crate) async fn decide_impl(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        info!(phase = "decide", "Evaluating safety and risk");

        let input_text = thought["input"].as_str().unwrap_or("").to_string();

        // Stage 1: Check policies + query memory for past similar actions (parallel)
        let (policy_check, aegis_check, memory_risk_ctx) = tokio::join!(
            // Policy check via Contract
            self.call_sister(
                SisterId::Contract,
                "policy_check",
                json!({
                    "action": input_text,
                    "plan": thought.get("plan"),
                }),
            ),
            // Safety validation via Aegis
            self.call_sister(
                SisterId::Aegis,
                "aegis_validate_complete",
                json!({
                    "action": input_text,
                    "plan": thought.get("plan"),
                    "intent": thought.get("intent"),
                }),
            ),
            // Memory risk assessment: query past similar actions and their outcomes
            self.call_sister(
                SisterId::Memory,
                "memory_search_semantic",
                json!({
                    "query": format!("risk outcome result of: {}", input_text),
                    "limit": 3,
                    "filter": { "metadata.phase": "learn" },
                }),
            ),
        );

        // Stage 3: Determine risk level from intent
        let risk_level = if let Some(intent) = thought.get("intent") {
            let actions = intent.get("actions")
                .and_then(|a| a.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            if actions.iter().any(|a| a.contains("Delete") || a.contains("System")) {
                "high"
            } else if actions.iter().any(|a| a.contains("Execute") || a.contains("Network")) {
                "medium"
            } else if actions.iter().any(|a| a.contains("Write") || a.contains("Modify")) {
                "low"
            } else {
                "none"
            }
        } else {
            "none"
        };

        // Stage 4: Gate decision
        let gate_decision = match risk_level {
            "high" | "critical" => "requires_approval",
            "medium" => "shadow_first",
            _ => "approved",
        };

        // Stage 5: Prepare action token via Identity if approved
        let identity_result = if gate_decision == "approved" || gate_decision == "shadow_first" {
            self.call_sister(
                SisterId::Identity,
                "action_sign",
                json!({
                    "action": input_text,
                    "risk_level": risk_level,
                }),
            )
            .await
        } else {
            json!({"status": "pending_approval"})
        };

        // Shadow simulation for medium+ risk via Aegis
        let shadow_result = if risk_level == "medium" || risk_level == "high" {
            self.call_sister(
                SisterId::Aegis,
                "aegis_shadow_execute",
                json!({
                    "action": input_text,
                    "plan": thought.get("plan"),
                }),
            )
            .await
        } else {
            json!(null)
        };

        Ok(json!({
            "input": input_text,
            "thought": thought,
            "policy_check": policy_check,
            "safety_check": aegis_check,
            "memory_risk_context": memory_risk_ctx,
            "risk_level": risk_level,
            "gate_decision": gate_decision,
            "action_token": identity_result,
            "shadow_result": shadow_result,
        }))
    }

    /// Assess risk of the decided action
    pub(crate) async fn assess_risk_impl(
        &self,
        decision: &serde_json::Value,
    ) -> Result<RiskAssessment, HydraError> {
        let risk_level = decision["risk_level"]
            .as_str()
            .unwrap_or("none");

        let level = match risk_level {
            "critical" => RiskLevel::Critical,
            "high" => RiskLevel::High,
            "medium" => RiskLevel::Medium,
            "low" => RiskLevel::Low,
            _ => RiskLevel::None,
        };

        Ok(RiskAssessment {
            level,
            factors: vec![],
            mitigations: vec![],
            requires_approval: matches!(level, RiskLevel::High | RiskLevel::Critical),
        })
    }
}
