//! AST nodes for compiled action sequences.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A node in the action AST
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionNode {
    /// Execute a single tool/action
    Action {
        tool: String,
        params: HashMap<String, ParamExpr>,
    },
    /// Execute a sequence of actions in order
    Sequence(Vec<ActionNode>),
    /// Conditional execution
    If {
        condition: ConditionExpr,
        then: Box<ActionNode>,
        #[serde(rename = "else")]
        else_: Option<Box<ActionNode>>,
    },
    /// Iterate over a collection
    ForEach {
        variable: String,
        collection: CollectionExpr,
        body: Box<ActionNode>,
    },
    /// Store result of an action for later use
    StoreResult {
        key: String,
        action: Box<ActionNode>,
    },
}

impl ActionNode {
    /// Count the number of leaf actions in this AST
    pub fn action_count(&self) -> usize {
        match self {
            Self::Action { .. } => 1,
            Self::Sequence(nodes) => nodes.iter().map(|n| n.action_count()).sum(),
            Self::If { then, else_, .. } => {
                then.action_count() + else_.as_ref().map(|e| e.action_count()).unwrap_or(0)
            }
            Self::ForEach { body, .. } => body.action_count(),
            Self::StoreResult { action, .. } => action.action_count(),
        }
    }

    /// Get all tool names referenced in this AST
    pub fn tool_names(&self) -> Vec<&str> {
        let mut names = Vec::new();
        self.collect_tool_names(&mut names);
        names
    }

    fn collect_tool_names<'a>(&'a self, names: &mut Vec<&'a str>) {
        match self {
            Self::Action { tool, .. } => names.push(tool),
            Self::Sequence(nodes) => {
                for node in nodes {
                    node.collect_tool_names(names);
                }
            }
            Self::If { then, else_, .. } => {
                then.collect_tool_names(names);
                if let Some(e) = else_ {
                    e.collect_tool_names(names);
                }
            }
            Self::ForEach { body, .. } => body.collect_tool_names(names),
            Self::StoreResult { action, .. } => action.collect_tool_names(names),
        }
    }
}

/// Parameter expression — how to compute a parameter value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamExpr {
    /// A fixed literal value
    Literal(serde_json::Value),
    /// A variable extracted from user input
    Variable(String),
    /// Result from a previous step
    PreviousResult(String),
    /// A computed transformation
    Computed(ComputeRule),
}

/// Condition for If nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionExpr {
    /// Check if a variable/result equals a value
    Equals {
        left: String,
        right: serde_json::Value,
    },
    /// Check if a result is not null/empty
    Exists(String),
    /// Check if a result indicates success
    Success(String),
    /// Boolean AND of conditions
    And(Vec<ConditionExpr>),
    /// Boolean OR of conditions
    Or(Vec<ConditionExpr>),
    /// Negate a condition
    Not(Box<ConditionExpr>),
}

/// Collection expression for ForEach
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionExpr {
    /// Literal array
    Literal(Vec<serde_json::Value>),
    /// From a previous result (expects array)
    FromResult(String),
    /// From a variable (expects array)
    FromVariable(String),
}

/// A transformation rule for computed parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeRule {
    /// Concatenate strings
    Concat(Vec<ParamExpr>),
    /// Format a template string
    Format {
        template: String,
        args: Vec<ParamExpr>,
    },
    /// Extract a field from a JSON value
    Extract { source: String, field: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_node() {
        let node = ActionNode::Action {
            tool: "git_commit".into(),
            params: HashMap::from([("message".into(), ParamExpr::Variable("commit_msg".into()))]),
        };
        assert_eq!(node.action_count(), 1);
        assert_eq!(node.tool_names(), vec!["git_commit"]);
    }

    #[test]
    fn test_sequence() {
        let node = ActionNode::Sequence(vec![
            ActionNode::Action {
                tool: "git_add".into(),
                params: HashMap::from([(
                    "path".into(),
                    ParamExpr::Literal(serde_json::json!(".")),
                )]),
            },
            ActionNode::Action {
                tool: "git_commit".into(),
                params: HashMap::from([("message".into(), ParamExpr::Variable("msg".into()))]),
            },
            ActionNode::Action {
                tool: "git_push".into(),
                params: HashMap::new(),
            },
        ]);
        assert_eq!(node.action_count(), 3);
        assert_eq!(node.tool_names(), vec!["git_add", "git_commit", "git_push"]);
    }

    #[test]
    fn test_if_condition() {
        let node = ActionNode::If {
            condition: ConditionExpr::Success("step_1".into()),
            then: Box::new(ActionNode::Action {
                tool: "deploy".into(),
                params: HashMap::new(),
            }),
            else_: Some(Box::new(ActionNode::Action {
                tool: "rollback".into(),
                params: HashMap::new(),
            })),
        };
        assert_eq!(node.action_count(), 2);
        assert_eq!(node.tool_names(), vec!["deploy", "rollback"]);
    }

    #[test]
    fn test_foreach() {
        let node = ActionNode::ForEach {
            variable: "file".into(),
            collection: CollectionExpr::Literal(vec![
                serde_json::json!("a.rs"),
                serde_json::json!("b.rs"),
            ]),
            body: Box::new(ActionNode::Action {
                tool: "lint".into(),
                params: HashMap::from([("path".into(), ParamExpr::Variable("file".into()))]),
            }),
        };
        assert_eq!(node.action_count(), 1); // body template counted once
        assert_eq!(node.tool_names(), vec!["lint"]);
    }

    #[test]
    fn test_store_result() {
        let node = ActionNode::StoreResult {
            key: "branch".into(),
            action: Box::new(ActionNode::Action {
                tool: "git_branch".into(),
                params: HashMap::new(),
            }),
        };
        assert_eq!(node.action_count(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let node = ActionNode::Sequence(vec![ActionNode::Action {
            tool: "test".into(),
            params: HashMap::from([
                ("a".into(), ParamExpr::Literal(serde_json::json!(42))),
                ("b".into(), ParamExpr::Variable("input".into())),
            ]),
        }]);
        let json = serde_json::to_string(&node).unwrap();
        let restored: ActionNode = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.action_count(), 1);
    }
}
