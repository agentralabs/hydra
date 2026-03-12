//! Plan extraction and deliverable step generation.

/// Extract a JSON execution plan from LLM response.
/// Looks for ```json ... ``` blocks and parses them.
pub fn extract_json_plan(response: &str) -> Option<serde_json::Value> {
    // Try to find ```json ... ``` block
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            let json_str = response[json_start..json_start + end].trim();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                if parsed.get("steps").is_some() {
                    return Some(parsed);
                }
            }
        }
    }

    // Fallback: try parsing the entire response as JSON
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response) {
        if parsed.get("steps").is_some() {
            return Some(parsed);
        }
    }

    None
}

/// Generate user-visible deliverable steps for complex tasks (NOT internal phases).
pub fn generate_deliverable_steps(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();

    // API/web project
    if lower.contains("api") || lower.contains("rest") || lower.contains("server") {
        let mut steps = vec!["Set up project structure".to_string()];
        if lower.contains("user") || lower.contains("account") {
            steps.push("Create user endpoints".to_string());
        }
        if lower.contains("auth") {
            steps.push("Add authentication".to_string());
        }
        if lower.contains("database") || lower.contains("db") {
            steps.push("Connect database".to_string());
        }
        steps.push("Write tests".to_string());
        return steps;
    }

    // Code modification
    if lower.contains("refactor") || lower.contains("modify") || lower.contains("fix") || lower.contains("update") {
        return vec![
            "Analyze current code".to_string(),
            "Apply changes".to_string(),
            "Verify results".to_string(),
        ];
    }

    // Code generation
    if lower.contains("create") || lower.contains("build") || lower.contains("implement") || lower.contains("generate") {
        return vec![
            "Plan structure".to_string(),
            "Generate code".to_string(),
            "Review and verify".to_string(),
        ];
    }

    // File operations
    if lower.contains("delete") || lower.contains("remove") || lower.contains("move") || lower.contains("rename") {
        return vec![
            "Identify targets".to_string(),
            "Execute operation".to_string(),
            "Confirm results".to_string(),
        ];
    }

    // Default multi-step
    vec![
        "Analyze request".to_string(),
        "Execute task".to_string(),
        "Verify outcome".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_plan_from_code_block() {
        let response = r#"Here's the plan:
```json
{"summary": "test", "steps": [{"type": "create_file", "path": "a.txt", "content": "hello"}]}
```
"#;
        let plan = extract_json_plan(response).unwrap();
        assert_eq!(plan["summary"], "test");
        assert!(plan["steps"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_extract_json_plan_raw() {
        let response = r#"{"steps": [{"type": "create_dir", "path": "src"}]}"#;
        let plan = extract_json_plan(response).unwrap();
        assert!(plan["steps"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn test_extract_json_plan_no_steps() {
        assert!(extract_json_plan("just some text").is_none());
        assert!(extract_json_plan(r#"{"foo": "bar"}"#).is_none());
    }

    #[test]
    fn test_generate_deliverable_steps_api() {
        let steps = generate_deliverable_steps("build a REST API with auth and database");
        assert!(steps.contains(&"Set up project structure".to_string()));
        assert!(steps.contains(&"Add authentication".to_string()));
        assert!(steps.contains(&"Connect database".to_string()));
    }

    #[test]
    fn test_generate_deliverable_steps_refactor() {
        let steps = generate_deliverable_steps("refactor the main module");
        assert_eq!(steps[0], "Analyze current code");
    }

    #[test]
    fn test_generate_deliverable_steps_default() {
        let steps = generate_deliverable_steps("do something unusual and long enough");
        assert_eq!(steps.len(), 3);
    }
}
