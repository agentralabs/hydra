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

    #[test]
    fn test_openclaw_type_mapping_extended() {
        assert_eq!(map_type("str"), ParamType::String);
        assert_eq!(map_type("int"), ParamType::Number);
        assert_eq!(map_type("float"), ParamType::Number);
        assert_eq!(map_type("boolean"), ParamType::Boolean);
        assert_eq!(map_type("dict"), ParamType::Object);
        assert_eq!(map_type("map"), ParamType::Object);
        assert_eq!(map_type("file"), ParamType::Path);
        assert_eq!(map_type("unknown"), ParamType::String);
    }

    #[test]
    fn test_openclaw_triggers() {
        let json = r#"{
            "name": "my_skill",
            "description": "desc",
            "inputs": {}
        }"#;
        let skill = OpenClawAdapter::parse(json).unwrap();
        assert!(skill.triggers.contains(&SkillTrigger::Intent("my_skill".into())));
        assert!(skill.triggers.contains(&SkillTrigger::Tool("openclaw.my_skill".into())));
    }

    #[test]
    fn test_openclaw_id_format() {
        let json = r#"{ "name": "abc", "description": "d", "inputs": {} }"#;
        let skill = OpenClawAdapter::parse(json).unwrap();
        assert_eq!(skill.id, "openclaw-abc");
    }

    #[test]
    fn test_openclaw_default_version() {
        let json = r#"{ "name": "x", "description": "d", "inputs": {} }"#;
        let skill = OpenClawAdapter::parse(json).unwrap();
        assert_eq!(skill.version, "1.0.0");
    }

    #[test]
    fn test_openclaw_custom_version() {
        let json = r#"{ "name": "x", "description": "d", "version": "2.0.0", "inputs": {} }"#;
        let skill = OpenClawAdapter::parse(json).unwrap();
        assert_eq!(skill.version, "2.0.0");
    }

    #[test]
    fn test_openclaw_risk_level() {
        let json = r#"{ "name": "x", "description": "d", "inputs": {} }"#;
        let skill = OpenClawAdapter::parse(json).unwrap();
        assert_eq!(skill.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_openclaw_parse_invalid() {
        let result = OpenClawAdapter::parse("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_openclaw_skill_serde() {
        let oc = OpenClawSkill {
            name: "test".into(),
            description: "desc".into(),
            version: "1.0.0".into(),
            inputs: HashMap::from([("in".into(), OpenClawParam { type_: "string".into(), description: "input".into(), required: true })]),
            outputs: HashMap::new(),
            tags: vec!["tag".into()],
        };
        let json = serde_json::to_string(&oc).unwrap();
        let restored: OpenClawSkill = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
    }
}
