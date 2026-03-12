//! Obstacle diagnoser — uses LLM multi-turn conversation to analyze obstacles.

use super::detector::{Obstacle, ObstaclePattern};

/// Diagnosis result from LLM analysis.
#[derive(Debug, Clone)]
pub struct Diagnosis {
    pub root_cause: String,
    pub affected_files: Vec<String>,
    pub suggested_approach: String,
    pub confidence: f32,
}

/// A fix strategy generated from diagnosis.
#[derive(Debug, Clone)]
pub struct Strategy {
    pub description: String,
    pub actions: Vec<FixAction>,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub enum FixAction {
    /// Modify an existing file with the given instruction.
    ModifyFile { path: String, instruction: String },
    /// Create a new file with the given content.
    CreateFile { path: String, content: String },
    /// Run a shell command (e.g., cargo add, mkdir).
    RunCommand { command: String },
    /// Add a dependency to Cargo.toml or package.json.
    AddDependency { name: String, version: Option<String> },
    /// Retry the original operation with different parameters.
    Retry { with_changes: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    Low,    // auto-apply
    Medium, // apply with checkpoint
    High,   // require user approval
}

/// Build an LLM prompt for obstacle diagnosis.
pub fn build_diagnosis_prompt(obstacle: &Obstacle) -> String {
    let pattern_hint = match obstacle.pattern {
        ObstaclePattern::CompilationError => {
            "This is a Rust compilation error. Focus on type mismatches, missing imports, \
             unresolved references, and syntax errors."
        }
        ObstaclePattern::TestFailure => {
            "This is a test failure. Focus on assertion mismatches, panics, and logic errors."
        }
        ObstaclePattern::MissingDependency => {
            "A dependency is missing. Identify which crate/package and suggest how to add it."
        }
        ObstaclePattern::FileNotFound => {
            "A file or path doesn't exist. Identify the expected path and whether to create or fix the reference."
        }
        ObstaclePattern::NetworkError => {
            "A network operation failed. Check if it's a DNS issue, connection refused, \
             or authentication problem."
        }
        ObstaclePattern::Timeout => {
            "An operation timed out. Suggest increasing the timeout or optimizing the operation."
        }
        ObstaclePattern::InvalidConfig => {
            "A configuration file has errors. Identify the exact field and correct value."
        }
        ObstaclePattern::PermissionDenied => {
            "A permission error occurred. Identify which resource and what permission is needed."
        }
        ObstaclePattern::Unknown => {
            "Analyze this error carefully and identify the root cause."
        }
    };

    let file_context = obstacle
        .source_file
        .as_deref()
        .map(|f| format!("\nAffected file: {}", f))
        .unwrap_or_default();

    format!(
        "I'm working on: {task}\n\n\
         I hit this error:\n```\n{error}\n```\n\
         {file_ctx}\n\n\
         {hint}\n\n\
         Respond in this exact JSON format:\n\
         ```json\n\
         {{\n  \
           \"root_cause\": \"one sentence explaining why this happened\",\n  \
           \"affected_files\": [\"path/to/file.rs\"],\n  \
           \"suggested_approach\": \"brief description of how to fix it\",\n  \
           \"confidence\": 0.8\n\
         }}\n\
         ```\n\
         Only output the JSON block, nothing else.",
        task = obstacle.task_context,
        error = truncate(&obstacle.error_message, 1500),
        file_ctx = file_context,
        hint = pattern_hint,
    )
}

/// Build an LLM prompt for generating fix strategies.
pub fn build_strategy_prompt(obstacle: &Obstacle, diagnosis: &Diagnosis) -> String {
    format!(
        "Root cause: {cause}\n\
         Affected files: {files}\n\
         Approach: {approach}\n\n\
         Original error:\n```\n{error}\n```\n\n\
         Generate 1-3 fix strategies as JSON. Each strategy has actions.\n\
         ```json\n\
         [{{\n  \
           \"description\": \"what this strategy does\",\n  \
           \"risk_level\": \"low\",\n  \
           \"actions\": [\n    \
             {{\"type\": \"modify_file\", \"path\": \"src/foo.rs\", \"instruction\": \"add missing import\"}},\n    \
             {{\"type\": \"run_command\", \"command\": \"cargo check\"}}\n  \
           ]\n\
         }}]\n\
         ```\n\
         Action types: modify_file, create_file, run_command, add_dependency, retry.\n\
         Only output the JSON array, nothing else.",
        cause = diagnosis.root_cause,
        files = diagnosis.affected_files.join(", "),
        approach = diagnosis.suggested_approach,
        error = truncate(&obstacle.error_message, 800),
    )
}

/// Build an LLM prompt for modifying an existing file (replaces single-shot append).
pub fn build_file_modify_prompt(file_path: &str, file_content: &str, instruction: &str) -> String {
    format!(
        "Here is the current content of `{path}`:\n\
         ```rust\n{content}\n```\n\n\
         Instruction: {instruction}\n\n\
         Return the COMPLETE modified file content. Do not append — \
         return the entire file with your changes integrated correctly. \
         Keep it under 400 lines. Only output the code block, no explanation.",
        path = file_path,
        content = truncate(file_content, 6000),
        instruction = instruction,
    )
}

/// Parse a diagnosis JSON from LLM response.
pub fn parse_diagnosis(response: &str) -> Result<Diagnosis, String> {
    let json_str = extract_json_block(response)?;
    let v: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    Ok(Diagnosis {
        root_cause: v["root_cause"].as_str().unwrap_or("unknown").to_string(),
        affected_files: v["affected_files"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        suggested_approach: v["suggested_approach"]
            .as_str()
            .unwrap_or("investigate further")
            .to_string(),
        confidence: v["confidence"].as_f64().unwrap_or(0.5) as f32,
    })
}

/// Parse strategy JSON array from LLM response.
pub fn parse_strategies(response: &str) -> Result<Vec<Strategy>, String> {
    let json_str = extract_json_block(response)?;
    let arr: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    let mut strategies = Vec::new();
    for item in arr.iter().take(3) {
        let risk_level = match item["risk_level"].as_str().unwrap_or("medium") {
            "low" => RiskLevel::Low,
            "high" => RiskLevel::High,
            _ => RiskLevel::Medium,
        };
        let actions = item["actions"]
            .as_array()
            .map(|acts| acts.iter().filter_map(parse_action).collect())
            .unwrap_or_default();

        strategies.push(Strategy {
            description: item["description"]
                .as_str()
                .unwrap_or("fix")
                .to_string(),
            actions,
            risk_level,
        });
    }
    Ok(strategies)
}

fn parse_action(v: &serde_json::Value) -> Option<FixAction> {
    match v["type"].as_str()? {
        "modify_file" => Some(FixAction::ModifyFile {
            path: v["path"].as_str()?.to_string(),
            instruction: v["instruction"].as_str()?.to_string(),
        }),
        "create_file" => Some(FixAction::CreateFile {
            path: v["path"].as_str()?.to_string(),
            content: v["content"].as_str().unwrap_or("").to_string(),
        }),
        "run_command" => Some(FixAction::RunCommand {
            command: v["command"].as_str()?.to_string(),
        }),
        "add_dependency" => Some(FixAction::AddDependency {
            name: v["name"].as_str()?.to_string(),
            version: v["version"].as_str().map(String::from),
        }),
        "retry" => Some(FixAction::Retry {
            with_changes: v["with_changes"].as_str().unwrap_or("").to_string(),
        }),
        _ => None,
    }
}

/// Extract a JSON block from an LLM response (handles ```json fences).
fn extract_json_block(response: &str) -> Result<String, String> {
    // Try to find ```json ... ``` block
    if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            return Ok(after[..end].trim().to_string());
        }
    }
    // Try ``` ... ``` block
    if let Some(start) = response.find("```") {
        let after = &response[start + 3..];
        if let Some(end) = after.find("```") {
            return Ok(after[..end].trim().to_string());
        }
    }
    // Try raw JSON
    let trimmed = response.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        return Ok(trimmed.to_string());
    }
    Err("No JSON found in response".to_string())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_diagnosis_prompt_compilation() {
        let obs = Obstacle::from_error("error[E0433]: unresolved import", "building");
        let prompt = build_diagnosis_prompt(&obs);
        assert!(prompt.contains("Rust compilation error"));
        assert!(prompt.contains("unresolved import"));
    }

    #[test]
    fn test_parse_diagnosis_valid() {
        let response = r#"```json
        {
            "root_cause": "missing import for HashMap",
            "affected_files": ["src/lib.rs"],
            "suggested_approach": "add use std::collections::HashMap",
            "confidence": 0.9
        }
        ```"#;
        let d = parse_diagnosis(response).unwrap();
        assert_eq!(d.root_cause, "missing import for HashMap");
        assert_eq!(d.affected_files, vec!["src/lib.rs"]);
        assert!(d.confidence > 0.8);
    }

    #[test]
    fn test_parse_diagnosis_raw_json() {
        let response = r#"{"root_cause":"timeout","affected_files":[],"suggested_approach":"retry","confidence":0.5}"#;
        let d = parse_diagnosis(response).unwrap();
        assert_eq!(d.root_cause, "timeout");
    }

    #[test]
    fn test_parse_strategies_valid() {
        let response = r#"```json
        [{"description":"add import","risk_level":"low","actions":[{"type":"modify_file","path":"src/lib.rs","instruction":"add HashMap import"}]}]
        ```"#;
        let strategies = parse_strategies(response).unwrap();
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].risk_level, RiskLevel::Low);
        assert_eq!(strategies[0].actions.len(), 1);
    }

    #[test]
    fn test_parse_strategies_max_three() {
        let response = r#"[
            {"description":"a","risk_level":"low","actions":[]},
            {"description":"b","risk_level":"medium","actions":[]},
            {"description":"c","risk_level":"high","actions":[]},
            {"description":"d","risk_level":"low","actions":[]}
        ]"#;
        let strategies = parse_strategies(response).unwrap();
        assert_eq!(strategies.len(), 3);
    }

    #[test]
    fn test_parse_action_types() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"type":"run_command","command":"cargo check"}"#,
        )
        .unwrap();
        let action = parse_action(&v).unwrap();
        assert!(matches!(action, FixAction::RunCommand { .. }));
    }

    #[test]
    fn test_extract_json_block_fenced() {
        let input = "Here's the fix:\n```json\n{\"a\":1}\n```\nDone.";
        assert_eq!(extract_json_block(input).unwrap(), "{\"a\":1}");
    }

    #[test]
    fn test_extract_json_block_raw() {
        assert_eq!(extract_json_block("[1,2,3]").unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_extract_json_block_no_json() {
        assert!(extract_json_block("no json here").is_err());
    }

    #[test]
    fn test_file_modify_prompt() {
        let prompt = build_file_modify_prompt("src/lib.rs", "fn main() {}", "add logging");
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("add logging"));
        assert!(prompt.contains("COMPLETE modified"));
    }
}
