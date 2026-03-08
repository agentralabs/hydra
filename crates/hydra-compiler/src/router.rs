//! ExecutionRouter — routes requests to compiled actions or LLM fallback.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::compiler::CompiledAction;
use crate::executor::{CompiledExecutor, ExecutionResult};

/// The routing decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingDecision {
    /// Execute via compiled AST (zero tokens)
    Compiled {
        compiled_id: String,
        signature: String,
    },
    /// Fall back to LLM (costs tokens)
    Llm { reason: String },
}

/// Routes execution to compiled actions when available, LLM otherwise
pub struct ExecutionRouter {
    /// Compiled actions indexed by signature
    compiled: parking_lot::Mutex<HashMap<String, CompiledAction>>,
    /// Stats
    compiled_hits: parking_lot::Mutex<u64>,
    llm_fallbacks: parking_lot::Mutex<u64>,
}

impl ExecutionRouter {
    pub fn new() -> Self {
        Self {
            compiled: parking_lot::Mutex::new(HashMap::new()),
            compiled_hits: parking_lot::Mutex::new(0),
            llm_fallbacks: parking_lot::Mutex::new(0),
        }
    }

    /// Register a compiled action
    pub fn register(&self, compiled: CompiledAction) {
        self.compiled
            .lock()
            .insert(compiled.signature.clone(), compiled);
    }

    /// Decide how to route a request
    pub fn route(&self, signature: &str) -> RoutingDecision {
        let compiled = self.compiled.lock();
        if let Some(action) = compiled.get(signature) {
            *self.compiled_hits.lock() += 1;
            RoutingDecision::Compiled {
                compiled_id: action.id.clone(),
                signature: action.signature.clone(),
            }
        } else {
            *self.llm_fallbacks.lock() += 1;
            RoutingDecision::Llm {
                reason: format!("No compiled action for signature: {}", signature),
            }
        }
    }

    /// Execute a compiled action directly (if available)
    pub fn execute_compiled(
        &self,
        signature: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> Option<ExecutionResult> {
        let compiled = self.compiled.lock();
        let action = compiled.get(signature)?;
        let mut executor = CompiledExecutor::with_variables(variables);
        Some(executor.execute(action))
    }

    /// Number of registered compiled actions
    pub fn compiled_count(&self) -> usize {
        self.compiled.lock().len()
    }

    /// Get routing stats
    pub fn stats(&self) -> RouterStats {
        RouterStats {
            compiled_actions: self.compiled.lock().len(),
            compiled_hits: *self.compiled_hits.lock(),
            llm_fallbacks: *self.llm_fallbacks.lock(),
        }
    }

    /// Remove a compiled action
    pub fn deregister(&self, signature: &str) -> bool {
        self.compiled.lock().remove(signature).is_some()
    }
}

impl Default for ExecutionRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStats {
    pub compiled_actions: usize,
    pub compiled_hits: u64,
    pub llm_fallbacks: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ActionNode;

    fn make_compiled(sig: &str) -> CompiledAction {
        CompiledAction {
            id: uuid::Uuid::new_v4().to_string(),
            signature: sig.into(),
            ast: ActionNode::Action {
                tool: "test".into(),
                params: HashMap::new(),
            },
            required_variables: vec![],
            compiled_at: chrono::Utc::now().to_rfc3339(),
            source_occurrences: 5,
            source_success_rate: 1.0,
        }
    }

    #[test]
    fn test_router_prefers_compiled() {
        let router = ExecutionRouter::new();
        router.register(make_compiled("git_push_flow"));

        let decision = router.route("git_push_flow");
        assert!(matches!(decision, RoutingDecision::Compiled { .. }));
    }

    #[test]
    fn test_router_fallback_to_llm() {
        let router = ExecutionRouter::new();
        let decision = router.route("unknown_action");
        assert!(matches!(decision, RoutingDecision::Llm { .. }));
    }

    #[test]
    fn test_router_execute_compiled() {
        let router = ExecutionRouter::new();
        router.register(make_compiled("deploy"));

        let result = router.execute_compiled("deploy", HashMap::new());
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.tokens_used, 0);
    }

    #[test]
    fn test_router_stats() {
        let router = ExecutionRouter::new();
        router.register(make_compiled("flow_a"));

        router.route("flow_a"); // compiled hit
        router.route("flow_a"); // compiled hit
        router.route("flow_b"); // LLM fallback

        let stats = router.stats();
        assert_eq!(stats.compiled_hits, 2);
        assert_eq!(stats.llm_fallbacks, 1);
        assert_eq!(stats.compiled_actions, 1);
    }

    #[test]
    fn test_router_deregister() {
        let router = ExecutionRouter::new();
        router.register(make_compiled("temp"));
        assert_eq!(router.compiled_count(), 1);
        assert!(router.deregister("temp"));
        assert_eq!(router.compiled_count(), 0);
    }

    #[test]
    fn test_router_default() {
        let router = ExecutionRouter::default();
        assert_eq!(router.compiled_count(), 0);
    }

    #[test]
    fn test_deregister_nonexistent() {
        let router = ExecutionRouter::new();
        assert!(!router.deregister("nope"));
    }

    #[test]
    fn test_execute_compiled_nonexistent() {
        let router = ExecutionRouter::new();
        let result = router.execute_compiled("nope", HashMap::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_routing_decision_serde() {
        let decision = RoutingDecision::Compiled { compiled_id: "id".into(), signature: "sig".into() };
        let json = serde_json::to_string(&decision).unwrap();
        let restored: RoutingDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, decision);
    }

    #[test]
    fn test_routing_decision_llm_serde() {
        let decision = RoutingDecision::Llm { reason: "no match".into() };
        let json = serde_json::to_string(&decision).unwrap();
        let restored: RoutingDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, decision);
    }

    #[test]
    fn test_router_stats_initial() {
        let router = ExecutionRouter::new();
        let stats = router.stats();
        assert_eq!(stats.compiled_actions, 0);
        assert_eq!(stats.compiled_hits, 0);
        assert_eq!(stats.llm_fallbacks, 0);
    }

    #[test]
    fn test_router_stats_serde() {
        let stats = RouterStats { compiled_actions: 5, compiled_hits: 10, llm_fallbacks: 3 };
        let json = serde_json::to_string(&stats).unwrap();
        let restored: RouterStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.compiled_hits, 10);
    }
}
