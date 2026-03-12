//! SkillDefinition — metadata, triggers, parameters, requirements.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Unique skill identifier
pub type SkillId = String;

/// A skill definition with metadata, contract, and execution info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: SkillId,
    pub name: String,
    pub version: String,
    pub description: String,
    pub triggers: Vec<SkillTrigger>,
    pub parameters: Vec<SkillParam>,
    pub outputs: Vec<SkillOutput>,
    pub requirements: Vec<Requirement>,
    pub source: SkillSource,
    pub sandbox_level: SandboxLevel,
    pub risk_level: RiskLevel,
    pub metadata: SkillMetadata,
}

/// How a skill can be triggered
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillTrigger {
    /// Natural language pattern: "create a * named *"
    Pattern(String),
    /// Intent classification: "file_creation"
    Intent(String),
    /// MCP tool name: "agentic_codebase.analyze"
    Tool(String),
}

/// A skill input parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParam {
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    pub description: String,
    pub default: Option<serde_json::Value>,
    pub constraints: Vec<Constraint>,
}

/// A skill output field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub name: String,
    pub output_type: ParamType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    MaxLength(usize),
    MinLength(usize),
    MaxValue(f64),
    MinValue(f64),
    Pattern(String),
    OneOf(Vec<serde_json::Value>),
}

/// What a skill needs to run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Requirement {
    Sister(String),
    Permission(String),
    Network,
    FileSystem,
    Environment(String),
}

/// Where the skill came from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillSource {
    Builtin,
    User,
    OpenClaw,
    Mcp { server: String },
}

/// Sandbox isolation level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SandboxLevel {
    /// Built-in only, full trust
    None,
    /// Network limited, temp filesystem only
    Basic,
    /// No filesystem, no network
    Strict,
}

/// Risk level for approval gating
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Skill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub author: String,
    pub license: String,
    pub tags: Vec<String>,
    pub cacheable: bool,
    pub idempotent: bool,
    pub reversible: bool,
    pub reverse_skill: Option<String>,
}

impl Default for SkillMetadata {
    fn default() -> Self {
        Self {
            author: String::new(),
            license: String::new(),
            tags: Vec::new(),
            cacheable: false,
            idempotent: false,
            reversible: false,
            reverse_skill: None,
        }
    }
}

impl SkillDefinition {
    /// Check if a trigger matches the given input
    pub fn matches_trigger(&self, input: &str) -> bool {
        self.triggers.iter().any(|t| match t {
            SkillTrigger::Pattern(pattern) => pattern_matches(pattern, input),
            SkillTrigger::Intent(intent) => input.eq_ignore_ascii_case(intent),
            SkillTrigger::Tool(tool) => input == tool,
        })
    }

    /// Get required sisters
    pub fn required_sisters(&self) -> Vec<&str> {
        self.requirements
            .iter()
            .filter_map(|r| match r {
                Requirement::Sister(s) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Check if skill needs network
    pub fn needs_network(&self) -> bool {
        self.requirements.contains(&Requirement::Network)
    }

    /// Validate inputs against parameter definitions
    pub fn validate_inputs(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for param in &self.parameters {
            if param.required && !inputs.contains_key(&param.name) {
                errors.push(format!("missing required parameter: {}", param.name));
                continue;
            }

            if let Some(value) = inputs.get(&param.name) {
                for constraint in &param.constraints {
                    if let Some(err) = validate_constraint(&param.name, value, constraint) {
                        errors.push(err);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Public version for use by registry
pub fn pattern_matches_pub(pattern: &str, input: &str) -> bool {
    pattern_matches(pattern, input)
}

/// Simple glob-like pattern matching (* matches any substring)
fn pattern_matches(pattern: &str, input: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern.eq_ignore_ascii_case(input);
    }

    let input_lower = input.to_lowercase();
    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let part_lower = part.to_lowercase();
        if i == 0 {
            if !input_lower.starts_with(&part_lower) {
                return false;
            }
            pos = part_lower.len();
        } else if let Some(found) = input_lower[pos..].find(&part_lower) {
            pos += found + part_lower.len();
        } else {
            return false;
        }
    }

    true
}

fn validate_constraint(
    name: &str,
    value: &serde_json::Value,
    constraint: &Constraint,
) -> Option<String> {
    match constraint {
        Constraint::MaxLength(max) => {
            if let Some(s) = value.as_str() {
                if s.len() > *max {
                    return Some(format!("{}: exceeds max length {}", name, max));
                }
            }
        }
        Constraint::MinLength(min) => {
            if let Some(s) = value.as_str() {
                if s.len() < *min {
                    return Some(format!("{}: below min length {}", name, min));
                }
            }
        }
        Constraint::MaxValue(max) => {
            if let Some(n) = value.as_f64() {
                if n > *max {
                    return Some(format!("{}: exceeds max value {}", name, max));
                }
            }
        }
        Constraint::MinValue(min) => {
            if let Some(n) = value.as_f64() {
                if n < *min {
                    return Some(format!("{}: below min value {}", name, min));
                }
            }
        }
        Constraint::Pattern(_regex) => {
            // Regex validation would go here
        }
        Constraint::OneOf(allowed) => {
            if !allowed.contains(value) {
                return Some(format!("{}: not in allowed values", name));
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "definition_tests.rs"]
mod tests;
