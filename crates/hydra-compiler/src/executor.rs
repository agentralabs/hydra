//! CompiledExecutor — runs compiled ASTs without LLM calls (zero tokens).

use std::collections::HashMap;
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

/// Executes compiled action ASTs without any LLM calls
pub struct CompiledExecutor {
    /// Context: variable bindings from user input
    variables: HashMap<String, serde_json::Value>,
    /// Results from previous steps (for StoreResult / PreviousResult)
    stored_results: HashMap<String, serde_json::Value>,
}

impl CompiledExecutor {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            stored_results: HashMap::new(),
        }
    }

    pub fn with_variables(variables: HashMap<String, serde_json::Value>) -> Self {
        Self {
            variables,
            stored_results: HashMap::new(),
        }
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
                // In production: call the actual tool via sister bridge
                // For now: simulate success
                let result = serde_json::json!({ "status": "ok", "tool": tool });
                results.push(StepResult {
                    tool: tool.clone(),
                    params: resolved,
                    result: result.clone(),
                    success: true,
                });
                true
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
}
