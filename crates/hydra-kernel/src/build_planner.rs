//! Build planner — analyzes a spec and produces a structured build plan.
//!
//! Uses LLM to classify complexity, identify crates, and order implementation steps.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Complexity classification for a build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Complexity {
    /// Modify/extend existing crate only.
    SingleCrate,
    /// Modify multiple existing crates.
    MultiCrate,
    /// Requires creating new crate(s).
    NewProject,
}

/// Spec for a crate to scaffold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateSpec {
    pub name: String,
    pub crate_type: String,
    pub is_new: bool,
    pub dependencies: Vec<(String, String)>,
    pub description: String,
}

/// Ordered implementation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationStep {
    pub crate_name: String,
    pub files: Vec<String>,
    pub description: String,
}

/// The complete build plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPlan {
    pub complexity: Complexity,
    pub crates: Vec<CrateSpec>,
    pub implementation_order: Vec<ImplementationStep>,
    pub test_strategy: String,
}

const BUILD_PLAN_PROMPT: &str = r#"You are planning a software build for Hydra, a Rust AI agent workspace.

Given the spec and workspace structure, generate a structured build plan as JSON.

{
  "complexity": "single_crate" | "multi_crate" | "new_project",
  "crates": [
    {
      "name": "crate-name",
      "crate_type": "lib",
      "is_new": true,
      "dependencies": [["serde", "{ workspace = true }"], ["tokio", "{ workspace = true }"]],
      "description": "what this crate does"
    }
  ],
  "implementation_order": [
    {"crate_name": "crate-name", "files": ["src/lib.rs", "src/types.rs"], "description": "core types and API"}
  ],
  "test_strategy": "inline"
}

Rules:
- Prefer extending existing crates over creating new ones
- New crates ONLY when the spec explicitly describes a standalone system
- Use workspace dependencies when the dep already exists in root Cargo.toml
- implementation_order determines batch sequence — put foundational code first
- Keep each step to 3-5 files max
- test_strategy: "inline" for small features, "suite" for multi-file systems
- Return ONLY the JSON object, no markdown fences
"#;

/// Generate a build plan from a spec using LLM.
pub async fn generate_build_plan(
    spec: &str,
    llm_config: &hydra_model::LlmConfig,
    project_dir: &Path,
) -> Result<BuildPlan, String> {
    let workspace_ctx = crate::self_modify_llm::gather_workspace_context(project_dir);
    let user_content = format!(
        "## Workspace Structure\n{}\n\n## Spec\n{}",
        workspace_ctx, spec
    );

    let response = crate::self_modify_llm::call_llm(
        &user_content,
        BUILD_PLAN_PROMPT,
        2000,
        llm_config,
    )
    .await?;

    parse_build_plan(&response)
}

/// Parse the LLM response into a BuildPlan.
fn parse_build_plan(response: &str) -> Result<BuildPlan, String> {
    let trimmed = response.trim();
    let json_str = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|s| s.strip_suffix("```"))
        .unwrap_or(trimmed);

    let value: serde_json::Value =
        serde_json::from_str(json_str.trim()).map_err(|e| format!("Invalid build plan JSON: {}", e))?;

    let complexity = match value
        .get("complexity")
        .and_then(|v| v.as_str())
        .unwrap_or("single_crate")
    {
        "multi_crate" => Complexity::MultiCrate,
        "new_project" => Complexity::NewProject,
        _ => Complexity::SingleCrate,
    };

    let crates: Vec<CrateSpec> = value
        .get("crates")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_crate_spec).collect())
        .unwrap_or_default();

    let implementation_order: Vec<ImplementationStep> = value
        .get("implementation_order")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_impl_step).collect())
        .unwrap_or_default();

    let test_strategy = value
        .get("test_strategy")
        .and_then(|v| v.as_str())
        .unwrap_or("inline")
        .to_string();

    if implementation_order.is_empty() {
        return Err("Build plan has no implementation steps".into());
    }

    Ok(BuildPlan {
        complexity,
        crates,
        implementation_order,
        test_strategy,
    })
}

fn parse_crate_spec(v: &serde_json::Value) -> Option<CrateSpec> {
    Some(CrateSpec {
        name: v.get("name")?.as_str()?.to_string(),
        crate_type: v
            .get("crate_type")
            .and_then(|t| t.as_str())
            .unwrap_or("lib")
            .to_string(),
        is_new: v.get("is_new").and_then(|b| b.as_bool()).unwrap_or(false),
        dependencies: v
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|pair| {
                        let a = pair.as_array()?;
                        Some((
                            a.first()?.as_str()?.to_string(),
                            a.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        description: v
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

fn parse_impl_step(v: &serde_json::Value) -> Option<ImplementationStep> {
    Some(ImplementationStep {
        crate_name: v.get("crate_name")?.as_str()?.to_string(),
        files: v
            .get("files")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        description: v
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_build_plan() {
        let json = r#"{
            "complexity": "multi_crate",
            "crates": [
                {
                    "name": "hydra-foo",
                    "crate_type": "lib",
                    "is_new": true,
                    "dependencies": [["serde", "{ workspace = true }"]],
                    "description": "Foo crate"
                }
            ],
            "implementation_order": [
                {"crate_name": "hydra-foo", "files": ["src/lib.rs"], "description": "core types"}
            ],
            "test_strategy": "suite"
        }"#;
        let plan = parse_build_plan(json).unwrap();
        assert_eq!(plan.complexity, Complexity::MultiCrate);
        assert_eq!(plan.crates.len(), 1);
        assert_eq!(plan.crates[0].name, "hydra-foo");
        assert!(plan.crates[0].is_new);
        assert_eq!(plan.implementation_order.len(), 1);
        assert_eq!(plan.test_strategy, "suite");
    }

    #[test]
    fn test_parse_build_plan_with_fences() {
        let json = "```json\n{\"complexity\":\"single_crate\",\"crates\":[],\"implementation_order\":[{\"crate_name\":\"hydra-kernel\",\"files\":[\"src/x.rs\"],\"description\":\"add x\"}],\"test_strategy\":\"inline\"}\n```";
        let plan = parse_build_plan(json).unwrap();
        assert_eq!(plan.complexity, Complexity::SingleCrate);
        assert_eq!(plan.implementation_order.len(), 1);
    }

    #[test]
    fn test_parse_build_plan_defaults() {
        let json = r#"{"implementation_order":[{"crate_name":"c","files":["f.rs"],"description":"d"}]}"#;
        let plan = parse_build_plan(json).unwrap();
        assert_eq!(plan.complexity, Complexity::SingleCrate);
        assert!(plan.crates.is_empty());
        assert_eq!(plan.test_strategy, "inline");
    }

    #[test]
    fn test_parse_build_plan_invalid() {
        let result = parse_build_plan("not json at all");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid build plan JSON"));
    }

    #[test]
    fn test_parse_build_plan_empty_steps() {
        let json = r#"{"implementation_order":[]}"#;
        let result = parse_build_plan(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no implementation steps"));
    }

    #[test]
    fn test_parse_crate_spec() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"name":"hydra-x","crate_type":"bin","is_new":true,"dependencies":[["tokio","{ workspace = true }"]],"description":"test"}"#,
        ).unwrap();
        let spec = parse_crate_spec(&v).unwrap();
        assert_eq!(spec.name, "hydra-x");
        assert_eq!(spec.crate_type, "bin");
        assert!(spec.is_new);
        assert_eq!(spec.dependencies.len(), 1);
        assert_eq!(spec.dependencies[0].0, "tokio");
    }

    #[test]
    fn test_parse_impl_step() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"crate_name":"hydra-kernel","files":["src/a.rs","src/b.rs"],"description":"impl"}"#,
        ).unwrap();
        let step = parse_impl_step(&v).unwrap();
        assert_eq!(step.crate_name, "hydra-kernel");
        assert_eq!(step.files.len(), 2);
        assert_eq!(step.description, "impl");
    }

    #[test]
    fn test_parse_impl_step_missing_crate() {
        let v: serde_json::Value = serde_json::from_str(r#"{"files":["a.rs"]}"#).unwrap();
        assert!(parse_impl_step(&v).is_none());
    }
}
