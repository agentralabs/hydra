//! Skill extractor — converts learned knowledge into stored beliefs/skills.

use super::api_learner::{ApiKnowledge, FixKnowledge};

/// A project's learned knowledge, structured for belief storage.
#[derive(Debug, Clone)]
pub struct ProjectKnowledge {
    pub project_name: String,
    pub purpose: String,
    pub setup_commands: Vec<String>,
    pub test_commands: Vec<String>,
    pub api_endpoints: Vec<super::api_learner::ApiEndpoint>,
    pub dependencies: Vec<String>,
    pub learned_at: chrono::DateTime<chrono::Utc>,
}

impl ProjectKnowledge {
    /// Parse from LLM JSON response.
    pub fn parse_from_llm(project_name: &str, response: &str) -> Self {
        let mut knowledge = Self {
            project_name: project_name.to_string(),
            purpose: String::new(),
            setup_commands: Vec::new(),
            test_commands: Vec::new(),
            api_endpoints: Vec::new(),
            dependencies: Vec::new(),
            learned_at: chrono::Utc::now(),
        };

        // Try JSON parsing
        if let Some(json_str) = extract_json(response) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                knowledge.purpose = val.get("purpose")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                knowledge.setup_commands = extract_string_array(&val, "setup_commands");
                knowledge.test_commands = extract_string_array(&val, "test_commands");
                knowledge.dependencies = extract_string_array(&val, "dependencies");

                if let Some(eps) = val.get("api_endpoints").and_then(|v| v.as_array()) {
                    for ep in eps.iter().take(20) {
                        knowledge.api_endpoints.push(super::api_learner::ApiEndpoint {
                            method: ep.get("method").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            path: ep.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            description: ep.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            params: extract_string_array(ep, "params"),
                        });
                    }
                }
            }
        }

        knowledge
    }

    /// One-line summary for LLM context.
    pub fn summary(&self) -> String {
        let mut parts = vec![];
        if !self.purpose.is_empty() {
            parts.push(format!("{}: {}", self.project_name, self.purpose));
        }
        if !self.test_commands.is_empty() {
            parts.push(format!("Test: {}", self.test_commands.join(", ")));
        }
        if !self.setup_commands.is_empty() {
            parts.push(format!("Setup: {}", self.setup_commands.join(", ")));
        }
        if parts.is_empty() {
            format!("Project: {}", self.project_name)
        } else {
            parts.join(" | ")
        }
    }
}

/// Convert ProjectKnowledge into belief tuples (subject, content, category).
pub fn project_as_beliefs(knowledge: &ProjectKnowledge) -> Vec<(String, String, String)> {
    let mut beliefs = Vec::new();
    let name = &knowledge.project_name;

    if !knowledge.purpose.is_empty() {
        beliefs.push((
            format!("{} purpose", name),
            knowledge.purpose.clone(),
            "knowledge".to_string(),
        ));
    }

    for cmd in &knowledge.setup_commands {
        beliefs.push((
            format!("{} setup", name),
            cmd.clone(),
            "knowledge".to_string(),
        ));
    }

    for cmd in &knowledge.test_commands {
        beliefs.push((
            format!("{} test", name),
            cmd.clone(),
            "knowledge".to_string(),
        ));
    }

    for dep in &knowledge.dependencies {
        beliefs.push((
            format!("{} dependency", name),
            dep.clone(),
            "knowledge".to_string(),
        ));
    }

    beliefs
}

/// Convert ApiKnowledge into belief tuples.
pub fn api_as_beliefs(knowledge: &ApiKnowledge) -> Vec<(String, String, String)> {
    let mut beliefs = Vec::new();
    let name = &knowledge.framework_name;

    for ep in &knowledge.endpoints {
        beliefs.push((
            format!("{} endpoint", name),
            format!("{} {} — {}", ep.method, ep.path, ep.description),
            "knowledge".to_string(),
        ));
    }

    if let Some(auth) = &knowledge.auth_method {
        beliefs.push((
            format!("{} auth", name),
            auth.clone(),
            "knowledge".to_string(),
        ));
    }

    if !knowledge.key_types.is_empty() {
        beliefs.push((
            format!("{} types", name),
            knowledge.key_types.join(", "),
            "knowledge".to_string(),
        ));
    }

    beliefs
}

/// Convert FixKnowledge into a belief tuple.
pub fn fix_as_belief(knowledge: &FixKnowledge) -> (String, String, String) {
    (
        format!("fix:{}", knowledge.error_pattern),
        format!("{} → {}", knowledge.root_cause, knowledge.fix_steps.join(" → ")),
        "knowledge".to_string(),
    )
}

/// Build the LLM prompt for learning a project from its README.
pub fn build_readme_learn_prompt(content: &str) -> String {
    format!(
        "Read this README and extract:\n\
         1. What this project does (one sentence)\n\
         2. How to install/setup\n\
         3. How to run tests\n\
         4. Key API endpoints or commands\n\
         5. Dependencies required\n\n\
         README:\n```\n{}\n```\n\n\
         Respond as JSON with fields: purpose, setup_commands, \
         test_commands, api_endpoints, dependencies",
        truncate(content, 6000)
    )
}

fn extract_json(response: &str) -> Option<String> {
    // ```json ... ```
    if let Some(start) = response.find("```json") {
        let rest = &response[start + 7..];
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    // ``` ... ```
    if let Some(start) = response.find("```") {
        let rest = &response[start + 3..];
        if let Some(end) = rest.find("```") {
            let block = rest[..end].trim();
            if block.starts_with('{') {
                return Some(block.to_string());
            }
        }
    }
    // Raw JSON
    let trimmed = response.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }
    None
}

fn extract_string_array(val: &serde_json::Value, key: &str) -> Vec<String> {
    val.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project_knowledge() {
        let response = r#"```json
{
  "purpose": "A multi-agent deployment framework",
  "setup_commands": ["cargo build"],
  "test_commands": ["cargo test"],
  "api_endpoints": [{"method": "POST", "path": "/deploy", "description": "Deploy agent", "params": ["name"]}],
  "dependencies": ["Rust 1.77+", "openssl"]
}
```"#;
        let k = ProjectKnowledge::parse_from_llm("xap-sdk", response);
        assert_eq!(k.purpose, "A multi-agent deployment framework");
        assert_eq!(k.setup_commands, vec!["cargo build"]);
        assert_eq!(k.test_commands, vec!["cargo test"]);
        assert_eq!(k.dependencies, vec!["Rust 1.77+", "openssl"]);
        assert_eq!(k.api_endpoints.len(), 1);
    }

    #[test]
    fn test_project_knowledge_summary() {
        let k = ProjectKnowledge {
            project_name: "myapp".into(),
            purpose: "web server".into(),
            setup_commands: vec!["npm install".into()],
            test_commands: vec!["npm test".into()],
            api_endpoints: vec![],
            dependencies: vec![],
            learned_at: chrono::Utc::now(),
        };
        let s = k.summary();
        assert!(s.contains("myapp"));
        assert!(s.contains("web server"));
        assert!(s.contains("npm test"));
    }

    #[test]
    fn test_project_as_beliefs() {
        let k = ProjectKnowledge {
            project_name: "sdk".into(),
            purpose: "deployment tool".into(),
            setup_commands: vec!["cargo build".into()],
            test_commands: vec!["cargo test".into()],
            api_endpoints: vec![],
            dependencies: vec!["openssl".into()],
            learned_at: chrono::Utc::now(),
        };
        let beliefs = project_as_beliefs(&k);
        assert_eq!(beliefs.len(), 4); // purpose + setup + test + dep
        assert!(beliefs.iter().any(|(s, c, _)| s.contains("purpose") && c == "deployment tool"));
        assert!(beliefs.iter().all(|(_, _, cat)| cat == "knowledge"));
    }

    #[test]
    fn test_api_as_beliefs() {
        let mut k = ApiKnowledge::empty("express");
        k.endpoints.push(super::super::api_learner::ApiEndpoint {
            method: "GET".into(), path: "/users".into(),
            description: "List users".into(), params: vec![],
        });
        k.auth_method = Some("bearer".into());
        k.key_types = vec!["User".into()];

        let beliefs = api_as_beliefs(&k);
        assert_eq!(beliefs.len(), 3); // endpoint + auth + types
    }

    #[test]
    fn test_fix_as_belief() {
        let fix = FixKnowledge {
            error_pattern: "ModuleNotFound".into(),
            root_cause: "missing dep".into(),
            fix_steps: vec!["pip install foo".into()],
            confidence: 0.9,
        };
        let (subj, content, cat) = fix_as_belief(&fix);
        assert!(subj.contains("ModuleNotFound"));
        assert!(content.contains("pip install foo"));
        assert_eq!(cat, "knowledge");
    }

    #[test]
    fn test_build_readme_prompt() {
        let p = build_readme_learn_prompt("# My Project\nDoes stuff");
        assert!(p.contains("My Project"));
        assert!(p.contains("purpose"));
        assert!(p.contains("setup_commands"));
    }

    #[test]
    fn test_parse_empty_response() {
        let k = ProjectKnowledge::parse_from_llm("test", "no json here");
        assert!(k.purpose.is_empty());
        assert!(k.setup_commands.is_empty());
    }
}
