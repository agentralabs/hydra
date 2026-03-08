//! CompiledExecutor — runs compiled ASTs without LLM calls (zero tokens).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::ast::{ActionNode, CollectionExpr, ConditionExpr, ParamExpr};
use crate::compiler::CompiledAction;

/// Result of executing a compiled action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub compiled_id: String,
    pub signature: String,
    pub success: bool,
    pub tokens_used: u64,
    pub duration_ms: u64,
    pub steps_executed: u32,
    pub results: Vec<StepResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub tool: String,
    pub params: HashMap<String, serde_json::Value>,
    pub result: serde_json::Value,
    pub success: bool,
}

/// Callback type for dispatching tool execution through a real bridge.
/// Arguments: (tool_name, resolved_params) -> Result<Value, error_message>
pub type CompiledToolDispatcher = Arc<
    dyn Fn(&str, &HashMap<String, serde_json::Value>) -> Result<serde_json::Value, String>
        + Send
        + Sync,
>;

/// Executes compiled action ASTs without any LLM calls
pub struct CompiledExecutor {
    /// Context: variable bindings from user input
    variables: HashMap<String, serde_json::Value>,
    /// Results from previous steps (for StoreResult / PreviousResult)
    stored_results: HashMap<String, serde_json::Value>,
    /// Optional dispatcher for real tool execution via sister bridges
    tool_dispatcher: Option<CompiledToolDispatcher>,
}

impl CompiledExecutor {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            stored_results: HashMap::new(),
            tool_dispatcher: None,
        }
    }

    pub fn with_variables(variables: HashMap<String, serde_json::Value>) -> Self {
        Self {
            variables,
            stored_results: HashMap::new(),
            tool_dispatcher: None,
        }
    }

    /// Set a real tool dispatcher for bridging compiled action execution
    /// through sister bridges instead of simulating results.
    pub fn with_dispatcher(mut self, dispatcher: CompiledToolDispatcher) -> Self {
        self.tool_dispatcher = Some(dispatcher);
        self
    }

    /// Execute a compiled action. Returns zero tokens used.
    pub fn execute(&mut self, compiled: &CompiledAction) -> ExecutionResult {
        let start = Instant::now();
        let mut results = Vec::new();

        let success = self.execute_node(&compiled.ast, &mut results);

        ExecutionResult {
            compiled_id: compiled.id.clone(),
            signature: compiled.signature.clone(),
            success,
            tokens_used: 0, // Zero tokens — that's the whole point
            duration_ms: start.elapsed().as_millis() as u64,
            steps_executed: results.len() as u32,
            results,
            error: if success {
                None
            } else {
                Some("Step failed".into())
            },
        }
    }

    fn execute_node(&mut self, node: &ActionNode, results: &mut Vec<StepResult>) -> bool {
        match node {
            ActionNode::Action { tool, params } => {
                let resolved = self.resolve_params(params);

                let (result, success) = if let Some(ref dispatcher) = self.tool_dispatcher {
                    // Dispatch through real sister bridge
                    match dispatcher(tool, &resolved) {
                        Ok(val) => (val, true),
                        Err(err) => (serde_json::json!({ "error": err }), false),
                    }
                } else {
                    // Fallback: simulate success
                    (serde_json::json!({ "status": "ok", "tool": tool }), true)
                };

                results.push(StepResult {
                    tool: tool.clone(),
                    params: resolved,
                    result: result.clone(),
                    success,
                });
                success
            }
            ActionNode::Sequence(nodes) => {
                for node in nodes {
                    if !self.execute_node(node, results) {
                        return false;
                    }
                }
                true
            }
            ActionNode::If {
                condition,
                then,
                else_,
            } => {
                if self.evaluate_condition(condition) {
                    self.execute_node(then, results)
                } else if let Some(else_node) = else_ {
                    self.execute_node(else_node, results)
                } else {
                    true
                }
            }
            ActionNode::ForEach {
                variable,
                collection,
                body,
            } => {
                let items = self.resolve_collection(collection);
                for item in items {
                    self.variables.insert(variable.clone(), item);
                    if !self.execute_node(body, results) {
                        return false;
                    }
                }
                true
            }
            ActionNode::StoreResult { key, action } => {
                let prev_len = results.len();
                let success = self.execute_node(action, results);
                if success {
                    if let Some(last) = results.get(prev_len) {
                        self.stored_results.insert(key.clone(), last.result.clone());
                    }
                }
                success
            }
        }
    }

    fn resolve_params(
        &self,
        params: &HashMap<String, ParamExpr>,
    ) -> HashMap<String, serde_json::Value> {
        params
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve_param(v)))
            .collect()
    }

    fn resolve_param(&self, expr: &ParamExpr) -> serde_json::Value {
        match expr {
            ParamExpr::Literal(v) => v.clone(),
            ParamExpr::Variable(name) => self
                .variables
                .get(name)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            ParamExpr::PreviousResult(key) => self
                .stored_results
                .get(key)
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            ParamExpr::Computed(_) => {
                // In production: evaluate compute rules
                serde_json::Value::Null
            }
        }
    }

    fn evaluate_condition(&self, condition: &ConditionExpr) -> bool {
        match condition {
            ConditionExpr::Exists(key) => {
                self.stored_results.contains_key(key) || self.variables.contains_key(key)
            }
            ConditionExpr::Success(key) => self
                .stored_results
                .get(key)
                .and_then(|v| v.get("status"))
                .and_then(|s| s.as_str())
                .map(|s| s == "ok")
                .unwrap_or(false),
            ConditionExpr::Equals { left, right } => {
                let left_val = self
                    .variables
                    .get(left)
                    .or_else(|| self.stored_results.get(left));
                left_val.map(|v| v == right).unwrap_or(false)
            }
            ConditionExpr::And(conditions) => conditions.iter().all(|c| self.evaluate_condition(c)),
            ConditionExpr::Or(conditions) => conditions.iter().any(|c| self.evaluate_condition(c)),
            ConditionExpr::Not(inner) => !self.evaluate_condition(inner),
        }
    }

    fn resolve_collection(&self, collection: &CollectionExpr) -> Vec<serde_json::Value> {
        match collection {
            CollectionExpr::Literal(items) => items.clone(),
            CollectionExpr::FromResult(key) => self
                .stored_results
                .get(key)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            CollectionExpr::FromVariable(key) => self
                .variables
                .get(key)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
        }
    }
}

impl Default for CompiledExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CompiledAction;

    fn make_compiled(ast: ActionNode) -> CompiledAction {
        CompiledAction {
            id: "test-1".into(),
            signature: "test".into(),
            ast,
            required_variables: vec![],
            compiled_at: chrono::Utc::now().to_rfc3339(),
            source_occurrences: 5,
            source_success_rate: 1.0,
        }
    }

    #[test]
    fn test_execute_single_action() {
        let compiled = make_compiled(ActionNode::Action {
            tool: "test_tool".into(),
            params: HashMap::from([("key".into(), ParamExpr::Literal(serde_json::json!("value")))]),
        });

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);

        assert!(result.success);
        assert_eq!(result.tokens_used, 0);
        assert_eq!(result.steps_executed, 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_zero_tokens() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::Action {
                tool: "a".into(),
                params: HashMap::new(),
            },
            ActionNode::Action {
                tool: "b".into(),
                params: HashMap::new(),
            },
            ActionNode::Action {
                tool: "c".into(),
                params: HashMap::new(),
            },
        ]));

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert_eq!(result.tokens_used, 0);
        assert_eq!(result.steps_executed, 3);
    }

    #[test]
    fn test_variable_resolution() {
        let compiled = make_compiled(ActionNode::Action {
            tool: "commit".into(),
            params: HashMap::from([("msg".into(), ParamExpr::Variable("user_msg".into()))]),
        });

        let mut executor = CompiledExecutor::with_variables(HashMap::from([(
            "user_msg".into(),
            serde_json::json!("fix: the bug"),
        )]));
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(
            result.results[0].params["msg"],
            serde_json::json!("fix: the bug")
        );
    }

    #[test]
    fn test_foreach_execution() {
        let compiled = make_compiled(ActionNode::ForEach {
            variable: "file".into(),
            collection: CollectionExpr::Literal(vec![
                serde_json::json!("a.rs"),
                serde_json::json!("b.rs"),
            ]),
            body: Box::new(ActionNode::Action {
                tool: "lint".into(),
                params: HashMap::from([("path".into(), ParamExpr::Variable("file".into()))]),
            }),
        });

        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
    }

    #[test]
    fn test_default_executor() {
        let executor = CompiledExecutor::default();
        let compiled = make_compiled(ActionNode::Action {
            tool: "t".into(),
            params: HashMap::new(),
        });
        let mut executor = executor;
        let result = executor.execute(&compiled);
        assert!(result.success);
    }

    #[test]
    fn test_store_result() {
        let compiled = make_compiled(ActionNode::StoreResult {
            key: "step1".into(),
            action: Box::new(ActionNode::Action {
                tool: "fetch".into(),
                params: HashMap::new(),
            }),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_if_condition_exists_true() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::StoreResult {
                key: "data".into(),
                action: Box::new(ActionNode::Action { tool: "fetch".into(), params: HashMap::new() }),
            },
            ActionNode::If {
                condition: ConditionExpr::Exists("data".into()),
                then: Box::new(ActionNode::Action { tool: "process".into(), params: HashMap::new() }),
                else_: None,
            },
        ]));
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2); // fetch + process
    }

    #[test]
    fn test_if_condition_exists_false() {
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Exists("nonexistent".into()),
            then: Box::new(ActionNode::Action { tool: "skip".into(), params: HashMap::new() }),
            else_: Some(Box::new(ActionNode::Action { tool: "fallback".into(), params: HashMap::new() })),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.results[0].tool, "fallback");
    }

    #[test]
    fn test_condition_not() {
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Not(Box::new(ConditionExpr::Exists("nope".into()))),
            then: Box::new(ActionNode::Action { tool: "yes".into(), params: HashMap::new() }),
            else_: None,
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
        assert_eq!(result.results[0].tool, "yes");
    }

    #[test]
    fn test_condition_and() {
        let mut executor = CompiledExecutor::with_variables(HashMap::from([
            ("a".into(), serde_json::json!("exists")),
            ("b".into(), serde_json::json!("also")),
        ]));
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::And(vec![
                ConditionExpr::Exists("a".into()),
                ConditionExpr::Exists("b".into()),
            ]),
            then: Box::new(ActionNode::Action { tool: "both".into(), params: HashMap::new() }),
            else_: None,
        });
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_condition_or() {
        let mut executor = CompiledExecutor::with_variables(HashMap::from([
            ("a".into(), serde_json::json!("yes")),
        ]));
        let compiled = make_compiled(ActionNode::If {
            condition: ConditionExpr::Or(vec![
                ConditionExpr::Exists("a".into()),
                ConditionExpr::Exists("z".into()),
            ]),
            then: Box::new(ActionNode::Action { tool: "found".into(), params: HashMap::new() }),
            else_: None,
        });
        let result = executor.execute(&compiled);
        assert_eq!(result.steps_executed, 1);
    }

    #[test]
    fn test_previous_result_param() {
        let compiled = make_compiled(ActionNode::Sequence(vec![
            ActionNode::StoreResult {
                key: "step1".into(),
                action: Box::new(ActionNode::Action { tool: "fetch".into(), params: HashMap::new() }),
            },
            ActionNode::Action {
                tool: "use".into(),
                params: HashMap::from([("data".into(), ParamExpr::PreviousResult("step1".into()))]),
            },
        ]));
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 2);
    }

    #[test]
    fn test_execution_result_serde() {
        let result = ExecutionResult {
            compiled_id: "id".into(),
            signature: "sig".into(),
            success: true,
            tokens_used: 0,
            duration_ms: 5,
            steps_executed: 1,
            results: vec![],
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tokens_used, 0);
    }

    #[test]
    fn test_foreach_empty_collection() {
        let compiled = make_compiled(ActionNode::ForEach {
            variable: "item".into(),
            collection: CollectionExpr::Literal(vec![]),
            body: Box::new(ActionNode::Action { tool: "process".into(), params: HashMap::new() }),
        });
        let mut executor = CompiledExecutor::new();
        let result = executor.execute(&compiled);
        assert!(result.success);
        assert_eq!(result.steps_executed, 0);
    }
}
