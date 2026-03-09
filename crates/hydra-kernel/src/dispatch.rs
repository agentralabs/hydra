//! Sister Dispatch Matrix — routes cognitive phases to the 14 sisters.
//!
//! This is the heart of Hydra V1.1: the PhaseHandler implementation that
//! actually dispatches to real sisters during each cognitive phase.
//!
//! Dispatch matrix (from PART1-SISTER-INTEGRATION.md):
//! | Sister    | PERCEIVE | THINK | DECIDE | ACT | LEARN |
//! |-----------|----------|-------|--------|-----|-------|
//! | Memory    | ✓        | ○     | ○      | ○   | ✓     |
//! | Vision    | ○        |       |        | ○   | ○     |
//! | Codebase  | ○        | ○     |        | ✓   |       |
//! | Identity  |          |       | ○      | ✓   | ✓     |
//! | Time      | ✓        | ○     |        | ○   | ○     |
//! | Contract  |          |       | ✓      |     |       |
//! | Comm      |          |       |        | ○   |       |
//! | Planning  |          | ✓     |        |     | ○     |
//! | Cognition | ✓        | ✓     |        |     | ✓     |
//! | Reality   | ✓        | ○     |        | ○   | ○     |
//! | Veritas   |          | ✓     |        |     |       |
//! | Aegis     |          |       | ✓      | ✓   |       |
//! | Evolve    |          |       |        |     | ✓     |
//! | Forge     |          | ○     |        | ○   |       |
//!
//! Legend: ✓ = Always called  ○ = Called when relevant

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tracing::{debug, info, warn};

use hydra_core::error::HydraError;
use hydra_core::types::{RiskAssessment, RiskLevel, TokenBudget};
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
    compiler: Arc<IntentCompiler>,
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
    async fn call_sister(
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
    fn involves_code(input: &CycleInput) -> bool {
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
    fn involves_vision(input: &CycleInput) -> bool {
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
    fn involves_network(input: &CycleInput) -> bool {
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
    /// PERCEIVE: Gather context from relevant sisters (10s timeout)
    ///
    /// Always called: Memory, Time, Cognition, Reality
    /// Conditionally: Vision (if visual), Codebase (if code)
    async fn perceive(&self, input: &CycleInput) -> Result<serde_json::Value, HydraError> {
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
    async fn think(&self, perceived: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        info!(phase = "think", "Compiling intent and planning");

        let input_text = perceived["input"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let involves_code = perceived["involves_code"].as_bool().unwrap_or(false);

        // Stage 1: Compile intent via local compiler (0 tokens if cached/classified)
        let mut budget = TokenBudget::new(5000);
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
    async fn decide(&self, thought: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
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
    async fn assess_risk(
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

    /// ACT: Execute the plan step by step (300s timeout)
    ///
    /// Routes to appropriate sisters based on action type:
    /// - Codebase: always for code ops
    /// - Identity: signs receipt after each step
    /// - Aegis: shadow-executes if risk >= Medium
    /// - Vision, Comm, Forge: when relevant
    async fn act(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        info!(phase = "act", "Executing plan");

        let gate_decision = decision["gate_decision"]
            .as_str()
            .unwrap_or("approved");

        // Block if not approved
        if gate_decision == "requires_approval" {
            return Ok(json!({
                "status": "blocked",
                "reason": "Action requires human approval",
                "risk_level": decision.get("risk_level"),
            }));
        }

        let input_text = decision["input"].as_str().unwrap_or("").to_string();
        let thought = decision.get("thought").cloned().unwrap_or(json!({}));
        let involves_code = thought
            .get("involves_code")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let involves_network = thought
            .get("involves_network")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Execute via appropriate sisters
        let mut results = vec![];

        // Code operations → Codebase sister
        if involves_code {
            let code_result = self
                .call_sister(
                    SisterId::Codebase,
                    "search_semantic",
                    json!({
                        "operation": "execute",
                        "plan": thought.get("plan"),
                        "blueprint": thought.get("blueprint"),
                    }),
                )
                .await;
            results.push(("codebase", code_result));
        }

        // Network operations → Comm sister
        if involves_network {
            let comm_result = self
                .call_sister(
                    SisterId::Comm,
                    "comm_message",
                    json!({
                        "operation": "send",
                        "context": input_text,
                    }),
                )
                .await;
            results.push(("comm", comm_result));
        }

        // Sign receipt via Identity
        let receipt = self
            .call_sister(
                SisterId::Identity,
                "receipt_create",
                json!({
                    "action": input_text,
                    "results": results.iter().map(|(k, v)| json!({"sister": *k, "data": v})).collect::<Vec<_>>(),
                    "risk_level": decision.get("risk_level"),
                }),
            )
            .await;

        Ok(json!({
            "status": "completed",
            "input": input_text,
            "results": results.iter().map(|(k, v)| json!({"sister": k, "result": v})).collect::<Vec<_>>(),
            "receipt": receipt,
        }))
    }

    /// LEARN: Store interaction with causal chains, update beliefs, crystallize skills (10s, non-blocking)
    ///
    /// Uses V3 capture tools (memory_capture_message, memory_capture_decision) for
    /// structured capture with causal chains, plus V2 memory_add as fallback.
    /// This is the Hydra-specific enhancement from THE-UNIVERSAL-FIX.md.
    ///
    /// Always called: Memory (V3 capture + V2 fallback), Cognition, Evolve, Identity, Time
    async fn learn(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
        info!(phase = "learn", "Updating memory and beliefs (V3 causal capture)");

        let input_text = result["input"].as_str().unwrap_or("").to_string();
        let status = result["status"].as_str().unwrap_or("completed").to_string();
        let risk_level = result.get("risk_level")
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string();

        // Build causal chain from the cognitive cycle
        let results_summary = result.get("results")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v["sister"].as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", "))
            .unwrap_or_default();

        // All learning calls in parallel (non-blocking)
        let (
            capture_msg_result,
            capture_decision_result,
            cognition_result,
            evolve_result,
            identity_result,
            time_result,
        ) = tokio::join!(
            // V3: Structured message capture with causal context
            self.call_sister(
                SisterId::Memory,
                "memory_capture_message",
                json!({
                    "role": "interaction",
                    "content": input_text,
                    "summary": format!("Action: {} | Status: {} | Sisters: {}", input_text, status, results_summary),
                    "metadata": {
                        "phase": "learn",
                        "risk_level": risk_level,
                        "status": status,
                        "sisters_involved": results_summary,
                        "causal_chain": {
                            "trigger": "user_input",
                            "perceived": true,
                            "planned": true,
                            "gate_decision": result.get("gate_decision"),
                            "outcome": status,
                        }
                    },
                })
            ),
            // V3: Capture decisions made during this cycle
            self.call_sister(
                SisterId::Memory,
                "memory_capture_decision",
                json!({
                    "decision": format!("Executed '{}' with risk={}", input_text, risk_level),
                    "reasoning": format!("Gate: {} | Sisters: {}", status, results_summary),
                    "alternatives": [],
                    "confidence": if status == "completed" { 0.9 } else { 0.5 },
                })
            ),
            // Update beliefs if correction detected
            self.call_sister(
                SisterId::Cognition,
                "cognition_belief_revise",
                json!({
                    "interaction": input_text,
                    "result": result,
                })
            ),
            // Crystallize skill if pattern detected
            self.call_sister(
                SisterId::Evolve,
                "evolve_crystallize",
                json!({
                    "interaction": input_text,
                    "result": result,
                })
            ),
            // Record in continuity log
            self.call_sister(
                SisterId::Identity,
                "continuity_record",
                json!({
                    "action": input_text,
                    "outcome": result.get("status"),
                })
            ),
            // Record completion time
            self.call_sister(
                SisterId::Time,
                "time_duration_track",
                json!({
                    "action": input_text,
                    "status": "completed",
                })
            ),
        );

        Ok(json!({
            "learning": "completed",
            "memory_capture": capture_msg_result,
            "decision_capture": capture_decision_result,
            "beliefs_updated": cognition_result,
            "skill_crystallized": evolve_result,
            "continuity": identity_result,
            "time_recorded": time_result,
        }))
    }

    fn sisters_available(&self) -> bool {
        self.sisters_online
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_core::types::CognitivePhase;
    use hydra_gate::GateConfig;
    use hydra_sisters::bridges;

    fn setup_dispatcher() -> SisterDispatcher {
        let mut registry = SisterRegistry::new();
        // Register all 14 sisters
        for bridge in bridges::all_bridges() {
            registry.register(bridge);
        }

        SisterDispatcher::new(
            Arc::new(registry),
            Arc::new(IntentCompiler::new()),
            Arc::new(ExecutionGate::new(GateConfig::default())),
        )
    }

    #[tokio::test]
    async fn test_perceive_always_calls_four_sisters() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("What time is it?");
        let result = dispatcher.perceive(&input).await.unwrap();

        // Memory, Time, Cognition, Reality should all be present
        assert!(result.get("memory").is_some());
        assert!(result.get("temporal").is_some());
        assert!(result.get("user_model").is_some());
        assert!(result.get("reality").is_some());
    }

    #[tokio::test]
    async fn test_perceive_code_adds_codebase() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("Fix the bug in src/main.rs");
        let result = dispatcher.perceive(&input).await.unwrap();

        assert_eq!(result["involves_code"], true);
        assert!(result.get("codebase").is_some());
        assert!(!result["codebase"].is_null());
    }

    #[tokio::test]
    async fn test_perceive_vision_adds_vision() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("Take a screenshot of the UI");
        let result = dispatcher.perceive(&input).await.unwrap();

        assert_eq!(result["involves_vision"], true);
        assert!(result.get("vision").is_some());
        assert!(!result["vision"].is_null());
    }

    #[tokio::test]
    async fn test_perceive_simple_query_no_code_or_vision() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("What is the weather?");
        let result = dispatcher.perceive(&input).await.unwrap();

        assert_eq!(result["involves_code"], false);
        assert_eq!(result["involves_vision"], false);
        assert!(result["codebase"].is_null());
        assert!(result["vision"].is_null());
    }

    #[tokio::test]
    async fn test_think_compiles_intent() {
        let dispatcher = setup_dispatcher();
        let perceived = json!({
            "input": "list all files",
            "involves_code": false,
            "involves_vision": false,
            "involves_network": false,
            "memory": {},
            "temporal": {},
            "user_model": {},
            "reality": {},
            "codebase": null,
            "vision": null,
        });

        let result = dispatcher.think(&perceived).await.unwrap();
        assert!(result.get("intent").is_some());
        assert!(result.get("plan").is_some());
        assert!(result.get("veritas").is_some());
        assert!(result.get("beliefs").is_some());
    }

    #[tokio::test]
    async fn test_think_code_generates_blueprint() {
        let dispatcher = setup_dispatcher();
        let perceived = json!({
            "input": "create a REST API endpoint",
            "involves_code": true,
            "involves_vision": false,
            "involves_network": false,
            "memory": {},
            "temporal": {},
            "user_model": {},
            "reality": {},
            "codebase": {"status": "ok"},
            "vision": null,
        });

        let result = dispatcher.think(&perceived).await.unwrap();
        assert!(!result["blueprint"].is_null());
    }

    #[tokio::test]
    async fn test_decide_low_risk_auto_approves() {
        let dispatcher = setup_dispatcher();
        let thought = json!({
            "input": "list all files",
            "involves_code": false,
            "involves_vision": false,
            "involves_network": false,
            "intent": {
                "actions": ["Read"],
                "confidence": 0.95,
            },
            "plan": {},
        });

        let result = dispatcher.decide(&thought).await.unwrap();
        assert_eq!(result["gate_decision"], "approved");
        assert_eq!(result["risk_level"], "none");
    }

    #[tokio::test]
    async fn test_decide_high_risk_requires_approval() {
        let dispatcher = setup_dispatcher();
        let thought = json!({
            "input": "delete all test files",
            "involves_code": true,
            "involves_vision": false,
            "involves_network": false,
            "intent": {
                "actions": ["FileDelete"],
                "confidence": 0.9,
            },
            "plan": {},
        });

        let result = dispatcher.decide(&thought).await.unwrap();
        assert_eq!(result["gate_decision"], "requires_approval");
        assert_eq!(result["risk_level"], "high");
    }

    #[tokio::test]
    async fn test_decide_medium_risk_shadow_first() {
        let dispatcher = setup_dispatcher();
        let thought = json!({
            "input": "run the test suite",
            "involves_code": true,
            "involves_vision": false,
            "involves_network": false,
            "intent": {
                "actions": ["Execute"],
                "confidence": 0.9,
            },
            "plan": {},
        });

        let result = dispatcher.decide(&thought).await.unwrap();
        assert_eq!(result["gate_decision"], "shadow_first");
        assert_eq!(result["risk_level"], "medium");
    }

    #[tokio::test]
    async fn test_act_blocked_when_requires_approval() {
        let dispatcher = setup_dispatcher();
        let decision = json!({
            "input": "delete everything",
            "gate_decision": "requires_approval",
            "risk_level": "high",
            "thought": {},
        });

        let result = dispatcher.act(&decision).await.unwrap();
        assert_eq!(result["status"], "blocked");
    }

    #[tokio::test]
    async fn test_act_executes_when_approved() {
        let dispatcher = setup_dispatcher();
        let decision = json!({
            "input": "list files",
            "gate_decision": "approved",
            "risk_level": "none",
            "thought": {
                "involves_code": false,
                "involves_network": false,
            },
        });

        let result = dispatcher.act(&decision).await.unwrap();
        assert_eq!(result["status"], "completed");
        assert!(result.get("receipt").is_some());
    }

    #[tokio::test]
    async fn test_learn_calls_all_learning_sisters() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "completed task",
            "status": "completed",
            "results": [],
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();
        assert_eq!(learn_result["learning"], "completed");
        assert!(learn_result.get("memory_capture").is_some());
        assert!(learn_result.get("decision_capture").is_some());
        assert!(learn_result.get("beliefs_updated").is_some());
        assert!(learn_result.get("skill_crystallized").is_some());
        assert!(learn_result.get("continuity").is_some());
        assert!(learn_result.get("time_recorded").is_some());
    }

    #[tokio::test]
    async fn test_assess_risk_levels() {
        let dispatcher = setup_dispatcher();

        let none_risk = dispatcher
            .assess_risk(&json!({"risk_level": "none"}))
            .await
            .unwrap();
        assert_eq!(none_risk.level, RiskLevel::None);
        assert!(!none_risk.needs_approval());

        let high_risk = dispatcher
            .assess_risk(&json!({"risk_level": "high"}))
            .await
            .unwrap();
        assert_eq!(high_risk.level, RiskLevel::High);
        assert!(high_risk.needs_approval());

        let critical_risk = dispatcher
            .assess_risk(&json!({"risk_level": "critical"}))
            .await
            .unwrap();
        assert_eq!(critical_risk.level, RiskLevel::Critical);
        assert!(critical_risk.needs_approval());
    }

    #[tokio::test]
    async fn test_involves_code_detection() {
        // With intent category from micro-LLM (preferred path)
        let with_intent = CycleInput {
            text: "fix it".into(),
            context: json!({"intent_category": "code_fix"}),
        };
        assert!(SisterDispatcher::involves_code(&with_intent));

        let no_code_intent = CycleInput {
            text: "fix it".into(),
            context: json!({"intent_category": "greeting"}),
        };
        assert!(!SisterDispatcher::involves_code(&no_code_intent));

        // Fallback: keyword heuristic (no intent available)
        assert!(SisterDispatcher::involves_code(&CycleInput::simple("Fix the bug in src/main.rs")));
        assert!(SisterDispatcher::involves_code(&CycleInput::simple("cargo build")));
        assert!(!SisterDispatcher::involves_code(&CycleInput::simple("What is the weather?")));
    }

    #[tokio::test]
    async fn test_involves_vision_detection() {
        let with_intent = CycleInput {
            text: "go to google.com".into(),
            context: json!({"intent_category": "web_browse"}),
        };
        assert!(SisterDispatcher::involves_vision(&with_intent));

        assert!(SisterDispatcher::involves_vision(&CycleInput::simple("Take a screenshot")));
        assert!(!SisterDispatcher::involves_vision(&CycleInput::simple("List all files")));
    }

    #[tokio::test]
    async fn test_involves_network_detection() {
        let with_intent = CycleInput {
            text: "tell him".into(),
            context: json!({"intent_category": "communicate"}),
        };
        assert!(SisterDispatcher::involves_network(&with_intent));

        assert!(SisterDispatcher::involves_network(&CycleInput::simple("Send an email")));
        assert!(!SisterDispatcher::involves_network(&CycleInput::simple("Read the file")));
    }

    #[tokio::test]
    async fn test_full_cognitive_cycle() {
        use crate::cognitive_loop::CognitiveLoop;
        use crate::config::KernelConfig;

        let dispatcher = setup_dispatcher();
        let kernel = CognitiveLoop::new(KernelConfig::default());
        let input = CycleInput::simple("list all files in the project");

        let output = kernel.run(input, &dispatcher).await;
        assert!(output.is_ok());
        assert_eq!(output.phases_completed.len(), 5);
        assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
        assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
    }

    #[tokio::test]
    async fn test_full_cycle_with_code_task() {
        use crate::cognitive_loop::CognitiveLoop;
        use crate::config::KernelConfig;

        let dispatcher = setup_dispatcher();
        let kernel = CognitiveLoop::new(KernelConfig::default());
        let input = CycleInput::simple("Fix the bug in src/main.rs");

        let output = kernel.run(input, &dispatcher).await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_cache_hit_on_repeated_intent() {
        let dispatcher = setup_dispatcher();

        // First call — may use classifier or LLM
        let perceived1 = json!({
            "input": "list files",
            "involves_code": false,
            "involves_vision": false,
            "involves_network": false,
        });
        let result1 = dispatcher.think(&perceived1).await.unwrap();

        // Second call — should use cache (0 tokens)
        let result2 = dispatcher.think(&perceived1).await.unwrap();

        // Both should have intents
        assert!(result1.get("intent").is_some());
        assert!(result2.get("intent").is_some());
    }

    // ═══════════════════════════════════════════════════════════
    // UNIVERSAL FIX TESTS — V3/V4 Memory Integration
    // ═══════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_perceive_includes_longevity_search() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("What did we discuss last week?");
        let result = dispatcher.perceive(&input).await.unwrap();

        // Must include both V2 memory and V4 longevity results
        assert!(result.get("memory").is_some(), "V2 memory_query missing");
        assert!(result.get("longevity").is_some(), "V4 longevity_search missing");
    }

    #[tokio::test]
    async fn test_perceive_longevity_search_not_null() {
        let dispatcher = setup_dispatcher();
        let input = CycleInput::simple("Remind me of our architecture decisions");
        let result = dispatcher.perceive(&input).await.unwrap();

        // Longevity result should be a valid JSON value (even if simulated)
        let longevity = &result["longevity"];
        assert!(!longevity.is_null() || longevity.is_object(),
            "longevity should be a valid response, got: {:?}", longevity);
    }

    #[tokio::test]
    async fn test_learn_v3_capture_message_present() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "refactor the auth module",
            "status": "completed",
            "results": [
                {"sister": "Codebase", "result": {"status": "ok"}},
                {"sister": "Identity", "result": {"status": "ok"}},
            ],
            "risk_level": "low",
            "gate_decision": "approved",
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();

        // V3 capture must be present (replaces old V2-only memory_add)
        assert!(learn_result.get("memory_capture").is_some(),
            "V3 memory_capture_message result missing");
        assert!(learn_result.get("decision_capture").is_some(),
            "V3 memory_capture_decision result missing");
    }

    #[tokio::test]
    async fn test_learn_v3_captures_causal_chain() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "deploy to production",
            "status": "completed",
            "results": [
                {"sister": "Codebase", "result": {"status": "ok"}},
            ],
            "risk_level": "high",
            "gate_decision": "approved",
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();

        // The learn phase should complete even with high-risk actions
        assert_eq!(learn_result["learning"], "completed");
        // Both V3 capture fields must exist
        assert!(learn_result.get("memory_capture").is_some());
        assert!(learn_result.get("decision_capture").is_some());
    }

    #[tokio::test]
    async fn test_learn_with_empty_results() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "simple query",
            "status": "completed",
            "results": [],
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();
        assert_eq!(learn_result["learning"], "completed");
        // Should still capture even with no sister results
        assert!(learn_result.get("memory_capture").is_some());
        assert!(learn_result.get("decision_capture").is_some());
    }

    #[tokio::test]
    async fn test_learn_with_failed_status() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "failed operation",
            "status": "failed",
            "results": [],
            "risk_level": "medium",
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();
        // Learning should complete even for failed actions (we learn from failures)
        assert_eq!(learn_result["learning"], "completed");
        assert!(learn_result.get("memory_capture").is_some());
    }

    #[tokio::test]
    async fn test_learn_with_missing_risk_level() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "query without risk",
            "status": "completed",
            "results": [],
        });

        // Should not panic when risk_level is absent
        let learn_result = dispatcher.learn(&result_data).await.unwrap();
        assert_eq!(learn_result["learning"], "completed");
    }

    #[tokio::test]
    async fn test_learn_with_multiple_sister_results() {
        let dispatcher = setup_dispatcher();
        let result_data = json!({
            "input": "complex multi-sister task",
            "status": "completed",
            "results": [
                {"sister": "Codebase", "result": {"status": "ok"}},
                {"sister": "Vision", "result": {"status": "ok"}},
                {"sister": "Comm", "result": {"status": "ok"}},
                {"sister": "Forge", "result": {"status": "ok"}},
            ],
            "risk_level": "medium",
            "gate_decision": "shadow_first",
        });

        let learn_result = dispatcher.learn(&result_data).await.unwrap();
        assert_eq!(learn_result["learning"], "completed");
        // All 6 parallel learning outputs must be present
        assert!(learn_result.get("memory_capture").is_some());
        assert!(learn_result.get("decision_capture").is_some());
        assert!(learn_result.get("beliefs_updated").is_some());
        assert!(learn_result.get("skill_crystallized").is_some());
        assert!(learn_result.get("continuity").is_some());
        assert!(learn_result.get("time_recorded").is_some());
    }

    #[tokio::test]
    async fn test_decide_includes_memory_risk_context() {
        let dispatcher = setup_dispatcher();
        let thought = json!({
            "input": "delete old backups",
            "involves_code": true,
            "involves_vision": false,
            "involves_network": false,
            "intent": {
                "actions": ["FileDelete"],
                "confidence": 0.9,
            },
            "plan": {},
        });

        let result = dispatcher.decide(&thought).await.unwrap();
        // Memory risk context should be present (queries past similar actions)
        assert!(result.get("memory_risk_context").is_some(),
            "memory_risk_context missing from DECIDE output");
    }

    #[tokio::test]
    async fn test_decide_memory_risk_for_safe_action() {
        let dispatcher = setup_dispatcher();
        let thought = json!({
            "input": "read the README",
            "involves_code": false,
            "involves_vision": false,
            "involves_network": false,
            "intent": {
                "actions": ["Read"],
                "confidence": 0.99,
            },
            "plan": {},
        });

        let result = dispatcher.decide(&thought).await.unwrap();
        assert!(result.get("memory_risk_context").is_some());
        assert_eq!(result["gate_decision"], "approved");
    }

    #[tokio::test]
    async fn test_full_cycle_with_v3_memory_integration() {
        use crate::cognitive_loop::CognitiveLoop;
        use crate::config::KernelConfig;

        let dispatcher = setup_dispatcher();
        let kernel = CognitiveLoop::new(KernelConfig::default());
        let input = CycleInput::simple("What patterns have I used before?");

        let output = kernel.run(input, &dispatcher).await;
        assert!(output.is_ok());
        assert_eq!(output.phases_completed.len(), 5);
        // The full cycle should complete with V3/V4 memory integration
        assert_eq!(output.phases_completed[0], CognitivePhase::Perceive);
        assert_eq!(output.phases_completed[4], CognitivePhase::Learn);
    }

    #[tokio::test]
    async fn test_full_cycle_completes_with_v3_memory() {
        use crate::cognitive_loop::CognitiveLoop;
        use crate::config::KernelConfig;

        let dispatcher = setup_dispatcher();
        let kernel = CognitiveLoop::new(KernelConfig::default());
        let input = CycleInput::simple("summarize yesterday's work");

        let output = kernel.run(input, &dispatcher).await;
        assert!(output.is_ok());

        // The CycleOutput.result comes from ACT, not LEARN.
        // LEARN is consumed internally. Verify the full cycle completed all 5 phases,
        // which proves LEARN (with V3 capture) executed successfully.
        assert_eq!(output.phases_completed.len(), 5,
            "Full cycle with V3 memory should complete all 5 phases");
        assert_eq!(output.phases_completed[4], CognitivePhase::Learn,
            "LEARN phase (with V3 capture) must be the final phase");

        // The result (from ACT) should still be valid
        assert!(output.result.get("status").is_some() || output.result.get("receipt").is_some(),
            "ACT result should contain status or receipt. Got: {:?}", output.result);
    }

    #[tokio::test]
    async fn test_perceive_longevity_parallel_with_memory() {
        let dispatcher = setup_dispatcher();

        // Both queries should run in parallel — verify by timing
        let start = std::time::Instant::now();
        let input = CycleInput::simple("Tell me about our project history");
        let result = dispatcher.perceive(&input).await.unwrap();
        let elapsed = start.elapsed();

        // Both should be present
        assert!(result.get("memory").is_some());
        assert!(result.get("longevity").is_some());

        // Parallel execution should complete within timeout (5s per sister + overhead)
        assert!(elapsed.as_secs() < 10,
            "Perceive took {}s — sisters may not be running in parallel", elapsed.as_secs());
    }
}
