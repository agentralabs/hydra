//! SequenceNormalizer — extracts variables and normalizes action sequences.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A normalized action sequence with variables extracted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSequence {
    /// Template actions with variables replaced by placeholders
    pub actions: Vec<NormalizedAction>,
    /// Extracted variable names and their sample values
    pub variables: HashMap<String, VariableInfo>,
    /// Signature for deduplication
    pub signature: String,
}

/// A single normalized action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedAction {
    pub tool: String,
    pub params: HashMap<String, NormalizedParam>,
}

/// A parameter that may be a literal or a variable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NormalizedParam {
    Literal(serde_json::Value),
    Variable { name: String },
}

/// Info about an extracted variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub sample_values: Vec<serde_json::Value>,
    pub inferred_type: InferredType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferredType {
    String,
    Number,
    Boolean,
    Path,
    Unknown,
}

/// Normalizes action sequences by extracting varying parameters as variables
pub struct SequenceNormalizer;

impl SequenceNormalizer {
    /// Normalize multiple instances of the same action sequence.
    /// Parameters that vary across instances become variables.
    pub fn normalize(instances: &[Vec<RawAction>]) -> Option<NormalizedSequence> {
        if instances.is_empty() {
            return None;
        }

        let first = &instances[0];
        if first.is_empty() {
            return None;
        }

        // All instances must have the same number of actions
        if !instances.iter().all(|i| i.len() == first.len()) {
            return None;
        }

        // All instances must use the same tools in the same order
        if !instances
            .iter()
            .all(|i| i.iter().zip(first.iter()).all(|(a, b)| a.tool == b.tool))
        {
            return None;
        }

        let mut variables: HashMap<String, VariableInfo> = HashMap::new();
        let mut normalized_actions = Vec::new();
        let mut sig_parts = Vec::new();
        let mut var_counter = 0u32;

        for (step_idx, template_action) in first.iter().enumerate() {
            let mut norm_params = HashMap::new();
            sig_parts.push(template_action.tool.clone());

            for (key, _) in &template_action.params {
                // Collect all values for this param across instances
                let values: Vec<&serde_json::Value> = instances
                    .iter()
                    .filter_map(|inst| inst.get(step_idx)?.params.get(key))
                    .collect();

                if values.is_empty() {
                    continue;
                }

                // Check if all values are the same
                let all_same = values.windows(2).all(|w| w[0] == w[1]);

                if all_same {
                    norm_params.insert(key.clone(), NormalizedParam::Literal(values[0].clone()));
                } else {
                    let var_name = format!("var_{}_{}", step_idx, var_counter);
                    var_counter += 1;

                    let inferred = infer_type(values[0]);
                    variables.insert(
                        var_name.clone(),
                        VariableInfo {
                            name: var_name.clone(),
                            sample_values: values.into_iter().cloned().collect(),
                            inferred_type: inferred,
                        },
                    );

                    norm_params.insert(key.clone(), NormalizedParam::Variable { name: var_name });
                }
            }

            normalized_actions.push(NormalizedAction {
                tool: template_action.tool.clone(),
                params: norm_params,
            });
        }

        let signature = sig_parts.join("→");

        Some(NormalizedSequence {
            actions: normalized_actions,
            variables,
            signature,
        })
    }
}

/// A raw action from execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAction {
    pub tool: String,
    pub params: HashMap<String, serde_json::Value>,
}

fn infer_type(value: &serde_json::Value) -> InferredType {
    match value {
        serde_json::Value::String(s) => {
            if s.contains('/') || s.contains('\\') || s.ends_with(".rs") || s.ends_with(".ts") {
                InferredType::Path
            } else {
                InferredType::String
            }
        }
        serde_json::Value::Number(_) => InferredType::Number,
        serde_json::Value::Bool(_) => InferredType::Boolean,
        _ => InferredType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_action(tool: &str, params: &[(&str, serde_json::Value)]) -> RawAction {
        RawAction {
            tool: tool.into(),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        }
    }

    #[test]
    fn test_normalize_constant_params() {
        let instances = vec![
            vec![make_action("git_add", &[("path", serde_json::json!("."))])],
            vec![make_action("git_add", &[("path", serde_json::json!("."))])],
        ];
        let norm = SequenceNormalizer::normalize(&instances).unwrap();
        assert_eq!(norm.actions.len(), 1);
        assert_eq!(
            norm.actions[0].params["path"],
            NormalizedParam::Literal(serde_json::json!("."))
        );
        assert!(norm.variables.is_empty());
    }

    #[test]
    fn test_normalize_variable_extraction() {
        let instances = vec![
            vec![make_action(
                "git_commit",
                &[("message", serde_json::json!("fix: bug A"))],
            )],
            vec![make_action(
                "git_commit",
                &[("message", serde_json::json!("feat: feature B"))],
            )],
            vec![make_action(
                "git_commit",
                &[("message", serde_json::json!("chore: cleanup"))],
            )],
        ];
        let norm = SequenceNormalizer::normalize(&instances).unwrap();
        assert_eq!(norm.variables.len(), 1);
        let var = norm.variables.values().next().unwrap();
        assert_eq!(var.sample_values.len(), 3);
        assert_eq!(var.inferred_type, InferredType::String);
    }

    #[test]
    fn test_normalize_mismatched_tools() {
        let instances = vec![
            vec![make_action("git_add", &[])],
            vec![make_action("git_commit", &[])],
        ];
        assert!(SequenceNormalizer::normalize(&instances).is_none());
    }

    #[test]
    fn test_normalize_signature() {
        let instances = vec![
            vec![
                make_action("git_add", &[("path", serde_json::json!("."))]),
                make_action("git_commit", &[("msg", serde_json::json!("a"))]),
            ],
            vec![
                make_action("git_add", &[("path", serde_json::json!("."))]),
                make_action("git_commit", &[("msg", serde_json::json!("b"))]),
            ],
        ];
        let norm = SequenceNormalizer::normalize(&instances).unwrap();
        assert_eq!(norm.signature, "git_add→git_commit");
    }

    #[test]
    fn test_path_type_inference() {
        let instances = vec![
            vec![make_action(
                "lint",
                &[("path", serde_json::json!("src/main.rs"))],
            )],
            vec![make_action(
                "lint",
                &[("path", serde_json::json!("src/lib.rs"))],
            )],
        ];
        let norm = SequenceNormalizer::normalize(&instances).unwrap();
        let var = norm.variables.values().next().unwrap();
        assert_eq!(var.inferred_type, InferredType::Path);
    }
}
