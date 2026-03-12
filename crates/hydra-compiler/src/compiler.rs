//! ActionCompiler — generates deterministic AST from normalized sequences.

use serde::{Deserialize, Serialize};

use crate::ast::{ActionNode, ParamExpr};
use crate::normalizer::{NormalizedParam, NormalizedSequence};

/// A compiled action ready for zero-token execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledAction {
    pub id: String,
    pub signature: String,
    pub ast: ActionNode,
    pub required_variables: Vec<String>,
    pub compiled_at: String,
    pub source_occurrences: u32,
    pub source_success_rate: f64,
}

/// Compiles normalized sequences into executable ASTs
pub struct ActionCompiler;

impl ActionCompiler {
    /// Compile a normalized sequence into an executable AST
    pub fn compile(
        normalized: &NormalizedSequence,
        occurrences: u32,
        success_rate: f64,
    ) -> CompiledAction {
        let required_variables: Vec<String> = normalized.variables.keys().cloned().collect();

        let ast = if normalized.actions.len() == 1 {
            Self::compile_single_action(&normalized.actions[0])
        } else {
            Self::compile_sequence(normalized)
        };

        CompiledAction {
            id: uuid::Uuid::new_v4().to_string(),
            signature: normalized.signature.clone(),
            ast,
            required_variables,
            compiled_at: chrono::Utc::now().to_rfc3339(),
            source_occurrences: occurrences,
            source_success_rate: success_rate,
        }
    }

    fn compile_single_action(action: &crate::normalizer::NormalizedAction) -> ActionNode {
        ActionNode::Action {
            tool: action.tool.clone(),
            params: action
                .params
                .iter()
                .map(|(k, v)| (k.clone(), Self::param_to_expr(v)))
                .collect(),
        }
    }

    fn compile_sequence(normalized: &NormalizedSequence) -> ActionNode {
        let nodes: Vec<ActionNode> = normalized
            .actions
            .iter()
            .map(|a| Self::compile_single_action(a))
            .collect();

        ActionNode::Sequence(nodes)
    }

    fn param_to_expr(param: &NormalizedParam) -> ParamExpr {
        match param {
            NormalizedParam::Literal(v) => ParamExpr::Literal(v.clone()),
            NormalizedParam::Variable { name } => ParamExpr::Variable(name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::{
        InferredType, NormalizedAction, NormalizedParam, NormalizedSequence, VariableInfo,
    };
    use std::collections::HashMap;

    fn simple_sequence() -> NormalizedSequence {
        NormalizedSequence {
            actions: vec![
                NormalizedAction {
                    tool: "git_add".into(),
                    params: HashMap::from([(
                        "path".into(),
                        NormalizedParam::Literal(serde_json::json!(".")),
                    )]),
                },
                NormalizedAction {
                    tool: "git_commit".into(),
                    params: HashMap::from([(
                        "message".into(),
                        NormalizedParam::Variable {
                            name: "var_0".into(),
                        },
                    )]),
                },
            ],
            variables: HashMap::from([(
                "var_0".into(),
                VariableInfo {
                    name: "var_0".into(),
                    sample_values: vec![serde_json::json!("fix: bug")],
                    inferred_type: InferredType::String,
                },
            )]),
            signature: "git_add→git_commit".into(),
        }
    }

    #[test]
    fn test_compile_simple() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 5, 1.0);
        assert_eq!(compiled.signature, "git_add→git_commit");
        assert_eq!(compiled.required_variables, vec!["var_0"]);
        assert_eq!(compiled.source_occurrences, 5);
        assert_eq!(compiled.ast.action_count(), 2);
    }

    #[test]
    fn test_compile_single_action() {
        let norm = NormalizedSequence {
            actions: vec![NormalizedAction {
                tool: "deploy".into(),
                params: HashMap::from([(
                    "env".into(),
                    NormalizedParam::Literal(serde_json::json!("prod")),
                )]),
            }],
            variables: HashMap::new(),
            signature: "deploy".into(),
        };
        let compiled = ActionCompiler::compile(&norm, 3, 1.0);
        assert_eq!(compiled.ast.action_count(), 1);
        assert!(compiled.required_variables.is_empty());
    }

    #[test]
    fn test_compiled_serializable() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 5, 1.0);
        let json = serde_json::to_string(&compiled).unwrap();
        assert!(json.contains("git_add"));
        assert!(json.contains("git_commit"));
    }

    #[test]
    fn test_compiled_action_has_id() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 1, 0.9);
        assert!(!compiled.id.is_empty());
    }

    #[test]
    fn test_compiled_action_has_timestamp() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 1, 1.0);
        assert!(!compiled.compiled_at.is_empty());
    }

    #[test]
    fn test_compiled_preserves_success_rate() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 10, 0.85);
        assert_eq!(compiled.source_success_rate, 0.85);
        assert_eq!(compiled.source_occurrences, 10);
    }

    #[test]
    fn test_compiled_action_serde_roundtrip() {
        let norm = simple_sequence();
        let compiled = ActionCompiler::compile(&norm, 3, 1.0);
        let json = serde_json::to_string(&compiled).unwrap();
        let restored: CompiledAction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.signature, "git_add→git_commit");
        assert_eq!(restored.ast.action_count(), 2);
    }

    #[test]
    fn test_compile_no_variables() {
        let norm = NormalizedSequence {
            actions: vec![NormalizedAction {
                tool: "test".into(),
                params: HashMap::from([("key".into(), NormalizedParam::Literal(serde_json::json!("val")))]),
            }],
            variables: HashMap::new(),
            signature: "test".into(),
        };
        let compiled = ActionCompiler::compile(&norm, 1, 1.0);
        assert!(compiled.required_variables.is_empty());
    }
}
