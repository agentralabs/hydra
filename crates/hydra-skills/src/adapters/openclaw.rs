//! OpenClaw adapter — import OpenClaw JSON/YAML skill format.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::definition::*;

/// OpenClaw skill format (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawSkill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
    pub inputs: HashMap<String, OpenClawParam>,
    #[serde(default)]
    pub outputs: HashMap<String, OpenClawParam>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawParam {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

/// Adapter for importing OpenClaw skills
pub struct OpenClawAdapter;

impl OpenClawAdapter {
    /// Parse an OpenClaw skill definition from JSON
    pub fn parse(json: &str) -> Result<SkillDefinition, String> {
        let oc: OpenClawSkill =
            serde_json::from_str(json).map_err(|e| format!("invalid OpenClaw JSON: {}", e))?;

        let parameters: Vec<SkillParam> = oc
            .inputs
            .iter()
            .map(|(name, param)| SkillParam {
                name: name.clone(),
                param_type: map_type(&param.type_),
                required: param.required,
                description: param.description.clone(),
                default: None,
                constraints: vec![],
            })
            .collect();

        let outputs: Vec<SkillOutput> = oc
            .outputs
            .iter()
            .map(|(name, param)| SkillOutput {
                name: name.clone(),
                output_type: map_type(&param.type_),
                description: param.description.clone(),
            })
            .collect();

        let id = format!("openclaw-{}", oc.name);

        Ok(SkillDefinition {
            id,
            name: oc.name.clone(),
            version: if oc.version.is_empty() {
                "1.0.0".into()
            } else {
                oc.version
            },
            description: oc.description,
            triggers: vec![
                SkillTrigger::Intent(oc.name.clone()),
                SkillTrigger::Tool(format!("openclaw.{}", oc.name)),
            ],
            parameters,
            outputs,
            requirements: vec![],
            source: SkillSource::OpenClaw,
            sandbox_level: SandboxLevel::Basic,
            risk_level: RiskLevel::Medium,
            metadata: SkillMetadata {
                tags: oc.tags,
                ..Default::default()
            },
        })
    }
}

fn map_type(type_str: &str) -> ParamType {
    match type_str.to_lowercase().as_str() {
        "string" | "str" => ParamType::String,
        "number" | "int" | "integer" | "float" => ParamType::Number,
        "boolean" | "bool" => ParamType::Boolean,
        "array" | "list" => ParamType::Array,
        "object" | "dict" | "map" => ParamType::Object,
        "path" | "file" => ParamType::Path,
        _ => ParamType::String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openclaw_adapter_parse() {
        let json = r#"{
            "name": "twitter_post",
            "description": "Post a tweet",
            "inputs": {
                "content": { "type": "string", "description": "Tweet text", "required": true },
                "media": { "type": "array", "description": "Media URLs", "required": false }
            },
            "outputs": {
                "tweet_id": { "type": "string", "description": "Tweet ID" }
            },
            "tags": ["social", "twitter"]
        }"#;

        let skill = OpenClawAdapter::parse(json).unwrap();
        assert_eq!(skill.name, "twitter_post");
        assert_eq!(skill.source, SkillSource::OpenClaw);
        assert_eq!(skill.sandbox_level, SandboxLevel::Basic);
        assert_eq!(skill.parameters.len(), 2);
        assert_eq!(skill.outputs.len(), 1);
        assert_eq!(skill.metadata.tags, vec!["social", "twitter"]);
    }

    #[test]
    fn test_openclaw_type_mapping() {
        assert_eq!(map_type("string"), ParamType::String);
        assert_eq!(map_type("integer"), ParamType::Number);
        assert_eq!(map_type("bool"), ParamType::Boolean);
        assert_eq!(map_type("list"), ParamType::Array);
        assert_eq!(map_type("path"), ParamType::Path);
    }
}
