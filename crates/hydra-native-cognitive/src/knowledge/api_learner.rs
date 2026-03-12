//! API learner — extracts API patterns from documentation via LLM prompts.

/// Knowledge extracted about an API or framework.
#[derive(Debug, Clone)]
pub struct ApiKnowledge {
    pub framework_name: String,
    pub endpoints: Vec<ApiEndpoint>,
    pub key_types: Vec<String>,
    pub auth_method: Option<String>,
    pub base_url_pattern: Option<String>,
    pub learned_at: chrono::DateTime<chrono::Utc>,
}

/// A single API endpoint or command.
#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
    pub params: Vec<String>,
}

/// Knowledge about how to fix a specific error using docs.
#[derive(Debug, Clone)]
pub struct FixKnowledge {
    pub error_pattern: String,
    pub root_cause: String,
    pub fix_steps: Vec<String>,
    pub confidence: f32,
}

impl ApiKnowledge {
    /// Create empty knowledge for a framework.
    pub fn empty(name: &str) -> Self {
        Self {
            framework_name: name.to_string(),
            endpoints: Vec::new(),
            key_types: Vec::new(),
            auth_method: None,
            base_url_pattern: None,
            learned_at: chrono::Utc::now(),
        }
    }

    /// Summary for LLM context injection.
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("Framework: {}", self.framework_name)];
        if !self.endpoints.is_empty() {
            let ep_list: Vec<String> = self.endpoints.iter()
                .take(5)
                .map(|e| format!("{} {}: {}", e.method, e.path, e.description))
                .collect();
            parts.push(format!("Endpoints: {}", ep_list.join("; ")));
        }
        if !self.key_types.is_empty() {
            parts.push(format!("Key types: {}", self.key_types.join(", ")));
        }
        if let Some(auth) = &self.auth_method {
            parts.push(format!("Auth: {}", auth));
        }
        parts.join("\n")
    }

    /// Parse API knowledge from LLM JSON response.
    pub fn parse_from_llm(name: &str, response: &str) -> Self {
        let mut knowledge = Self::empty(name);

        // Try JSON parsing first
        if let Some(json_str) = extract_json_block(response) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(endpoints) = val.get("endpoints").and_then(|v| v.as_array()) {
                    for ep in endpoints.iter().take(20) {
                        knowledge.endpoints.push(ApiEndpoint {
                            method: ep.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_string(),
                            path: ep.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            description: ep.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            params: ep.get("params").and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|p| p.as_str().map(String::from)).collect())
                                .unwrap_or_default(),
                        });
                    }
                }
                if let Some(types) = val.get("key_types").and_then(|v| v.as_array()) {
                    knowledge.key_types = types.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .take(20)
                        .collect();
                }
                knowledge.auth_method = val.get("auth_method")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
                knowledge.base_url_pattern = val.get("base_url")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(String::from);
            }
        }

        knowledge
    }
}

impl FixKnowledge {
    /// Parse fix knowledge from LLM JSON response.
    pub fn parse_from_llm(response: &str) -> Option<Self> {
        let json_str = extract_json_block(response)?;
        let val: serde_json::Value = serde_json::from_str(&json_str).ok()?;

        Some(Self {
            error_pattern: val.get("error_pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            root_cause: val.get("root_cause")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            fix_steps: val.get("fix_steps")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            confidence: val.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32,
        })
    }

    /// Summary for context injection.
    pub fn summary(&self) -> String {
        format!(
            "Error: {} → Cause: {} → Fix: {}",
            self.error_pattern,
            self.root_cause,
            self.fix_steps.join(" → ")
        )
    }
}

/// Build the LLM prompt for learning an API from documentation.
pub fn build_api_learn_prompt(doc_content: &str) -> String {
    format!(
        "Read this documentation and extract the API structure.\n\
         Return JSON with fields:\n\
         - endpoints: [{{method, path, description, params}}]\n\
         - key_types: [\"TypeName\", ...]\n\
         - auth_method: \"bearer\" | \"api_key\" | \"oauth\" | null\n\
         - base_url: \"https://...\" | null\n\n\
         Documentation:\n```\n{}\n```\n\n\
         Respond ONLY with the JSON object.",
        truncate(doc_content, 6000)
    )
}

/// Build the LLM prompt for fixing an error using documentation.
pub fn build_error_fix_prompt(error: &str, docs: &str) -> String {
    format!(
        "I got this error:\n```\n{}\n```\n\n\
         The relevant documentation says:\n```\n{}\n```\n\n\
         Return JSON with fields:\n\
         - error_pattern: the error type\n\
         - root_cause: why this happened\n\
         - fix_steps: [\"step 1\", \"step 2\", ...]\n\
         - confidence: 0.0-1.0\n\n\
         Respond ONLY with the JSON object.",
        truncate(error, 2000),
        truncate(docs, 4000)
    )
}

/// Extract JSON block from LLM response (handles ```json fences).
fn extract_json_block(response: &str) -> Option<String> {
    // Try ```json ... ``` first
    if let Some(start) = response.find("```json") {
        let rest = &response[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    // Try ``` ... ```
    if let Some(start) = response.find("```") {
        let rest = &response[start + 3..];
        if let Some(end) = rest.find("```") {
            let block = rest[..end].trim();
            if block.starts_with('{') || block.starts_with('[') {
                return Some(block.to_string());
            }
        }
    }
    // Try raw JSON
    let trimmed = response.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    None
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_knowledge_empty() {
        let k = ApiKnowledge::empty("express");
        assert_eq!(k.framework_name, "express");
        assert!(k.endpoints.is_empty());
    }

    #[test]
    fn test_api_knowledge_summary() {
        let mut k = ApiKnowledge::empty("express");
        k.endpoints.push(ApiEndpoint {
            method: "GET".into(), path: "/users".into(),
            description: "List users".into(), params: vec![],
        });
        k.auth_method = Some("bearer".into());
        let s = k.summary();
        assert!(s.contains("express"));
        assert!(s.contains("/users"));
        assert!(s.contains("bearer"));
    }

    #[test]
    fn test_parse_api_from_json() {
        let response = r#"```json
{"endpoints": [{"method": "POST", "path": "/api/deploy", "description": "Deploy app", "params": ["name"]}], "key_types": ["App", "Config"], "auth_method": "api_key", "base_url": "https://api.example.com"}
```"#;
        let k = ApiKnowledge::parse_from_llm("myapi", response);
        assert_eq!(k.endpoints.len(), 1);
        assert_eq!(k.endpoints[0].method, "POST");
        assert_eq!(k.key_types, vec!["App", "Config"]);
        assert_eq!(k.auth_method, Some("api_key".to_string()));
    }

    #[test]
    fn test_parse_api_raw_json() {
        let response = r#"{"endpoints": [], "key_types": ["Foo"], "auth_method": null, "base_url": null}"#;
        let k = ApiKnowledge::parse_from_llm("test", response);
        assert_eq!(k.key_types, vec!["Foo"]);
        assert!(k.auth_method.is_none());
    }

    #[test]
    fn test_parse_fix_knowledge() {
        let response = r#"```json
{"error_pattern": "ModuleNotFound", "root_cause": "missing dependency", "fix_steps": ["pip install requests"], "confidence": 0.9}
```"#;
        let fix = FixKnowledge::parse_from_llm(response).unwrap();
        assert_eq!(fix.error_pattern, "ModuleNotFound");
        assert_eq!(fix.fix_steps, vec!["pip install requests"]);
        assert!((fix.confidence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_fix_knowledge_summary() {
        let fix = FixKnowledge {
            error_pattern: "ImportError".into(),
            root_cause: "missing package".into(),
            fix_steps: vec!["install it".into(), "restart".into()],
            confidence: 0.8,
        };
        let s = fix.summary();
        assert!(s.contains("ImportError"));
        assert!(s.contains("missing package"));
    }

    #[test]
    fn test_build_api_learn_prompt() {
        let p = build_api_learn_prompt("# My API\nGET /foo");
        assert!(p.contains("endpoints"));
        assert!(p.contains("My API"));
    }

    #[test]
    fn test_build_error_fix_prompt() {
        let p = build_error_fix_prompt("command not found", "use `brew install`");
        assert!(p.contains("command not found"));
        assert!(p.contains("brew install"));
    }

    #[test]
    fn test_extract_json_block_raw() {
        let raw = r#"  {"key": "val"}  "#;
        assert_eq!(extract_json_block(raw).unwrap(), r#"{"key": "val"}"#);
    }

    #[test]
    fn test_extract_json_block_none() {
        assert!(extract_json_block("no json here").is_none());
    }
}
