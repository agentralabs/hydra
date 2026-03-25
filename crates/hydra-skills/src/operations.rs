//! Operational Skills — TOML-defined action plans that the conductor executes.
//! Bridges knowledge (genome) with action (conductor steps).
//! Skills become executable, not just advisory.

use std::collections::HashMap;
use serde::Deserialize;

// ── Types ──

/// A parsed operation from operations.toml.
#[derive(Debug, Clone)]
pub struct Operation {
    pub name: String,
    pub trigger: String,
    pub confidence: f64,
    pub steps: Vec<OperationStep>,
    pub params: Vec<OperationParam>,
}

/// A single step in an operation.
#[derive(Debug, Clone, Deserialize)]
pub struct OperationStep {
    #[serde(rename = "type")]
    pub step_type: String,
    pub command: Option<String>,
    pub description: Option<String>,
    pub target: Option<String>,
    pub template: Option<String>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub long_running: bool,
    pub wait_for: Option<String>,
    pub url: Option<String>,
    pub method: Option<String>,
    pub expect: Option<serde_json::Value>,
    pub path: Option<String>,
    pub text: Option<String>,
    pub action: Option<String>,
    pub goal: Option<String>,
    pub app: Option<String>,
    pub port: Option<u16>,
    pub timeout_ms: Option<u64>,
}

/// A parameter definition for an operation.
#[derive(Debug, Clone)]
pub struct OperationParam {
    pub name: String,
    pub param_type: String,
    pub default: Option<String>,
    pub required: bool,
    pub prompt: Option<String>,
}

// ── TOML Parsing ──

#[derive(Deserialize)]
struct OperationsFile {
    #[serde(default)]
    operation: Vec<RawOperation>,
}

#[derive(Deserialize)]
struct RawOperation {
    name: String,
    trigger: Option<String>,
    #[serde(default = "default_conf")]
    confidence: f64,
    #[serde(default)]
    steps: Vec<OperationStep>,
    #[serde(default)]
    params: HashMap<String, serde_json::Value>,
}

fn default_conf() -> f64 { 0.8 }

/// Load operations from a skill's operations.toml file.
pub fn load_operations(path: &str) -> Result<Vec<Operation>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read: {e}"))?;
    parse_operations(&content)
}

/// Parse operations from TOML content (EC-4.3: validate syntax).
pub fn parse_operations(toml_content: &str) -> Result<Vec<Operation>, String> {
    let parsed: OperationsFile = toml::from_str(toml_content)
        .map_err(|e| format!("TOML parse error: {e}"))?;

    parsed.operation.into_iter().map(|raw| {
        // Validate step types (EC-4.4)
        for step in &raw.steps {
            validate_step_type(&step.step_type)?;
        }
        // Parse params
        let params = raw.params.into_iter().map(|(name, val)| {
            parse_param(&name, &val)
        }).collect();

        Ok(Operation {
            name: raw.name,
            trigger: raw.trigger.unwrap_or_default(),
            confidence: raw.confidence,
            steps: raw.steps,
            params,
        })
    }).collect()
}

/// EC-4.8: Check if an operation contains destructive shell commands.
const DESTRUCTIVE_PATTERNS: &[&str] = &[
    "rm -rf", "rm -r /", "truncate ", "dd if=/dev/zero", "mkfs", "fdisk",
    "format c:", "> /dev/sda", "drop table", "drop database",
];

pub fn has_destructive_command(op: &Operation) -> bool {
    op.steps.iter().any(|s| {
        let cmd = s.command.as_deref().unwrap_or("").to_lowercase();
        DESTRUCTIVE_PATTERNS.iter().any(|p| cmd.contains(p))
    })
}

/// Load all operations from ~/.hydra/skills/*/operations.toml.
pub fn load_all_operations() -> Vec<Operation> {
    let mut all = Vec::new();
    let dir = dirs::home_dir().unwrap_or_default().join(".hydra/skills");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path().join("operations.toml");
            if path.exists() {
                match load_operations(&path.display().to_string()) {
                    Ok(ops) => all.extend(ops),
                    Err(e) => eprintln!("hydra-skills: load operations {}: {e}", path.display()),
                }
            }
        }
    }
    all
}

/// Match a user goal against operation triggers (fuzzy).
pub fn match_operation<'a>(goal: &str, operations: &'a [Operation]) -> Option<&'a Operation> {
    let lower = goal.to_lowercase();
    operations.iter()
        .filter(|op| {
            op.trigger.split('|').any(|t| lower.contains(t.trim()))
        })
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))
}

// ── Parameter Extraction ──

/// Extract parameters from user goal text + defaults (EC-4.1).
pub fn extract_params(
    goal: &str,
    param_defs: &[OperationParam],
) -> Result<HashMap<String, String>, Vec<String>> {
    let mut params = HashMap::new();
    let mut missing = Vec::new();

    for def in param_defs {
        // Try to extract from goal text (naive: use whole goal minus trigger words)
        let value = extract_param_from_text(goal, &def.name)
            .or_else(|| def.default.clone());

        match value {
            Some(v) => { params.insert(def.name.clone(), v); }
            None if def.required => { missing.push(def.name.clone()); }
            None => { params.insert(def.name.clone(), String::new()); }
        }
    }

    if missing.is_empty() { Ok(params) } else { Err(missing) }
}

fn extract_param_from_text(goal: &str, _param_name: &str) -> Option<String> {
    // Heuristic: extract quoted strings or the main subject
    if let Some(start) = goal.find('"') {
        if let Some(end) = goal[start + 1..].find('"') {
            return Some(goal[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

// ── Template Substitution ──

/// Substitute {{param}} in a string. EC-4.2: shell-escape for shell contexts.
pub fn substitute(template: &str, params: &HashMap<String, String>, shell_escape: bool) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        let pattern = format!("{{{{{key}}}}}");
        let replacement = if shell_escape { shell_escape_value(value) } else { value.clone() };
        result = result.replace(&pattern, &replacement);
    }
    result
}

fn shell_escape_value(value: &str) -> String {
    // EC-4.2: quote values with spaces or special chars
    if value.contains(' ') || value.contains('$') || value.contains('`') || value.contains('"') {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.to_string()
    }
}

// ── Validation ──

fn validate_step_type(step_type: &str) -> Result<(), String> {
    let valid = ["shell", "code_gen", "browser", "desktop", "file", "api", "wait", "verify", "conditional"];
    if valid.contains(&step_type) { Ok(()) }
    else { Err(format!("Unknown step type '{}'. Valid: {}", step_type, valid.join(", "))) }
}

fn parse_param(name: &str, val: &serde_json::Value) -> OperationParam {
    if let Some(obj) = val.as_object() {
        OperationParam {
            name: name.to_string(),
            param_type: obj.get("type").and_then(|t| t.as_str()).unwrap_or("string").to_string(),
            default: obj.get("default").map(|d| d.to_string().trim_matches('"').to_string()),
            required: obj.get("required").and_then(|r| r.as_bool()).unwrap_or(false),
            prompt: obj.get("prompt").and_then(|p| p.as_str()).map(|s| s.to_string()),
        }
    } else {
        OperationParam {
            name: name.to_string(), param_type: "string".into(),
            default: Some(val.to_string()), required: false, prompt: None,
        }
    }
}

// NOTE: Conductor bridge (to_conductor_steps) lives in hydra-kernel/src/conductor.rs
// to avoid circular dependency (kernel depends on skills, not vice versa).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_operation() {
        let toml = r#"
        [[operation]]
        name = "test_op"
        trigger = "test|run test"
        confidence = 0.9

        [[operation.steps]]
        type = "shell"
        command = "echo {{name}}"
        description = "Say hello"

        [operation.params]
        name = { type = "string", default = "world", required = false }
        "#;
        let ops = parse_operations(toml).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "test_op");
        assert_eq!(ops[0].steps.len(), 1);
    }

    #[test]
    fn invalid_step_type_rejected() {
        let toml = r#"
        [[operation]]
        name = "bad"
        [[operation.steps]]
        type = "kubernetes"
        "#;
        assert!(parse_operations(toml).is_err());
    }

    #[test]
    fn template_substitution() {
        let mut params = HashMap::new();
        params.insert("name".into(), "world".into());
        assert_eq!(substitute("echo {{name}}", &params, false), "echo world");
    }

    #[test]
    fn shell_escape_spaces() {
        let mut params = HashMap::new();
        params.insert("name".into(), "hello world".into());
        let result = substitute("echo {{name}}", &params, true);
        assert!(result.contains("'hello world'"));
    }

    #[test]
    fn match_trigger() {
        let ops = vec![
            Operation { name: "deploy".into(), trigger: "deploy|ship".into(), confidence: 0.9,
                steps: vec![], params: vec![] },
        ];
        assert!(match_operation("deploy to production", &ops).is_some());
        assert!(match_operation("cook dinner", &ops).is_none());
    }

    #[test]
    fn missing_required_param() {
        let defs = vec![OperationParam {
            name: "topic".into(), param_type: "string".into(),
            default: None, required: true, prompt: Some("Topic?".into()),
        }];
        let result = extract_params("do something", &defs);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(&"topic".to_string()));
    }

    #[test]
    fn default_param_used() {
        let defs = vec![OperationParam {
            name: "name".into(), param_type: "string".into(),
            default: Some("default-name".into()), required: false, prompt: None,
        }];
        let params = extract_params("do something", &defs).unwrap();
        assert_eq!(params.get("name").unwrap(), "default-name");
    }
}
