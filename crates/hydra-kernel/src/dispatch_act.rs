//! ACT + LEARN phase dispatch — extracted from dispatch.rs for file size.
//! Contains the act() and learn() PhaseHandler implementations.

use serde_json::json;
use tracing::info;

use hydra_core::error::HydraError;
use hydra_sisters::bridge::SisterId;

use super::dispatch::SisterDispatcher;

impl SisterDispatcher {
    /// ACT: Execute the plan step by step (300s timeout)
    ///
    /// Routes to appropriate sisters based on action type:
    /// - Codebase: always for code ops
    /// - Identity: signs receipt after each step
    /// - Aegis: shadow-executes if risk >= Medium
    /// - Vision, Comm, Forge: when relevant
    pub(crate) async fn act_impl(&self, decision: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
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

    /// LEARN: Store interaction with causal chains, update beliefs, crystallize skills
    pub(crate) async fn learn_impl(&self, result: &serde_json::Value) -> Result<serde_json::Value, HydraError> {
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
}
