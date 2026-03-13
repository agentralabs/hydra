//! Planning Agent — counterfactual reasoning, task chains, consensus,
//! dream processing, and advanced goal management.
//!
//! Extends planning_deep.rs (which has: create_goal, update_progress,
//! complete_goal, list_active, create_project_plan, checkpoint_phase)
//! and extras_deep.rs (which has: decompose_goal, identify_themes,
//! commitments_due_soon) with counterfactual, dream, metamorphosis,
//! chain, consensus, federation, context logging, evidence, grounding,
//! suggestion, and decision recording.

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // COUNTERFACTUAL & DREAM PROCESSING
    // ═══════════════════════════════════════════════════════════════

    /// "What if?" analysis — explore alternative scenarios and their outcomes.
    /// Used for risk assessment and decision support.
    pub async fn planning_counterfactual(&self, scenario: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_counterfactual", serde_json::json!({
            "scenario": safe_truncate(scenario, 500),
            "depth": 3,
            "include_alternatives": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Idle-state dream processing — background plan synthesis when not busy.
    /// Planning sister generates speculative plans that may be useful later.
    pub async fn planning_dream(&self, context: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_dream", serde_json::json!({
            "context": safe_truncate(context, 300),
            "mode": "speculative",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // GOAL RESTRUCTURING
    // ═══════════════════════════════════════════════════════════════

    /// Metamorphosis — restructure a goal when circumstances change.
    /// Splits, merges, or re-prioritizes sub-goals dynamically.
    pub async fn planning_metamorphosis(&self, goal_id: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_metamorphosis", serde_json::json!({
            "goal_id": goal_id,
            "operation": "restructure",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // TASK CHAINING & MULTI-AGENT CONSENSUS
    // ═══════════════════════════════════════════════════════════════

    /// Chain multiple tasks with dependency ordering.
    /// Planning sister resolves the execution graph.
    pub async fn planning_chain(&self, tasks: &[String]) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_chain", serde_json::json!({
            "tasks": tasks,
            "resolve_dependencies": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Multi-agent consensus — gather agreement on a proposal.
    /// Each agent votes and provides reasoning; Planning synthesizes.
    pub async fn planning_consensus(
        &self,
        proposal: &str,
        agents: &[String],
    ) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_consensus", serde_json::json!({
            "proposal": safe_truncate(proposal, 400),
            "agents": agents,
            "require_reasoning": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // FEDERATION & CONTEXT
    // ═══════════════════════════════════════════════════════════════

    /// Federate a task across distributed planning instances.
    pub async fn planning_federate(&self, task: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_federate", serde_json::json!({
            "task": safe_truncate(task, 300),
            "mode": "distribute",
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Log planning context — append to the running context trail.
    pub async fn planning_context_log(&self, context: &str) {
        if let Some(planning) = &self.planning {
            let _ = planning.call_tool("planning_context_log", serde_json::json!({
                "context": safe_truncate(context, 500),
            })).await;
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // EVIDENCE & GROUNDING
    // ═══════════════════════════════════════════════════════════════

    /// Gather evidence supporting or contradicting a claim within a plan.
    pub async fn planning_evidence(&self, claim: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_evidence", serde_json::json!({
            "claim": safe_truncate(claim, 300),
            "include_contradictions": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Ground a plan in reality — validate feasibility against known constraints.
    pub async fn planning_ground(&self, plan: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_ground", serde_json::json!({
            "plan": safe_truncate(plan, 500),
            "check_constraints": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // SUGGESTIONS & DECISIONS
    // ═══════════════════════════════════════════════════════════════

    /// Generate planning suggestions based on current context.
    pub async fn planning_suggest(&self, context: &str) -> Option<String> {
        let planning = self.planning.as_ref()?;
        let result = planning.call_tool("planning_suggest", serde_json::json!({
            "context": safe_truncate(context, 400),
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Record a decision with its reasoning for future reference.
    pub async fn planning_decision(&self, decision: &str, reasoning: &str) {
        if let Some(planning) = &self.planning {
            let _ = planning.call_tool("planning_decision", serde_json::json!({
                "decision": safe_truncate(decision, 300),
                "reasoning": safe_truncate(reasoning, 300),
            })).await;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_planning_agent_compiles() {
        assert!(true);
    }
}
