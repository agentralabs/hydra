#[cfg(test)]
mod tests {
    use crate::definition::*;
    use std::collections::HashMap;

    fn test_skill() -> SkillDefinition {
        SkillDefinition {
            id: "skill-1".into(),
            name: "tweet_post".into(),
            version: "1.0.0".into(),
            description: "Post a tweet".into(),
            triggers: vec![
                SkillTrigger::Pattern("post a * to twitter".into()),
                SkillTrigger::Intent("social_post".into()),
                SkillTrigger::Tool("twitter.post".into()),
            ],
            parameters: vec![SkillParam {
                name: "content".into(),
                param_type: ParamType::String,
                required: true,
                description: "Tweet content".into(),
                default: None,
                constraints: vec![Constraint::MaxLength(280)],
            }],
            outputs: vec![SkillOutput {
                name: "tweet_id".into(),
                output_type: ParamType::String,
                description: "ID of posted tweet".into(),
            }],
            requirements: vec![Requirement::Network, Requirement::Sister("vision".into())],
            source: SkillSource::Builtin,
            sandbox_level: SandboxLevel::Basic,
            risk_level: RiskLevel::Medium,
            metadata: SkillMetadata {
                author: "hydra".into(),
                license: "MIT".into(),
                tags: vec!["social".into(), "twitter".into()],
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_skill_definition_serde() {
        let skill = test_skill();
        let json = serde_json::to_string(&skill).unwrap();
        let restored: SkillDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "tweet_post");
        assert_eq!(restored.version, "1.0.0");
        assert_eq!(restored.triggers.len(), 3);
        assert_eq!(restored.sandbox_level, SandboxLevel::Basic);
    }

    #[test]
    fn test_skill_param_types() {
        let skill = test_skill();
        assert_eq!(skill.parameters.len(), 1);
        assert_eq!(skill.parameters[0].param_type, ParamType::String);
        assert!(skill.parameters[0].required);
    }

    #[test]
    fn test_skill_trigger_matching() {
        let skill = test_skill();
        assert!(skill.matches_trigger("post a message to twitter"));
        assert!(skill.matches_trigger("social_post"));
        assert!(skill.matches_trigger("twitter.post"));
        assert!(!skill.matches_trigger("send an email"));
    }

    #[test]
    fn test_validate_inputs() {
        let skill = test_skill();

        // Missing required
        let empty: HashMap<String, serde_json::Value> = HashMap::new();
        assert!(skill.validate_inputs(&empty).is_err());

        // Valid
        let valid = HashMap::from([("content".into(), serde_json::json!("hello"))]);
        assert!(skill.validate_inputs(&valid).is_ok());

        // Exceeds max length
        let long = HashMap::from([("content".into(), serde_json::json!("x".repeat(300)))]);
        assert!(skill.validate_inputs(&long).is_err());
    }

    #[test]
    fn test_required_sisters() {
        let skill = test_skill();
        assert_eq!(skill.required_sisters(), vec!["vision"]);
        assert!(skill.needs_network());
    }

    #[test]
    fn test_pattern_matches_exact() {
        assert!(pattern_matches_pub("hello world", "hello world"));
        assert!(!pattern_matches_pub("hello world", "goodbye"));
    }

    #[test]
    fn test_pattern_matches_case_insensitive() {
        assert!(pattern_matches_pub("Hello World", "hello world"));
    }

    #[test]
    fn test_pattern_matches_wildcard() {
        assert!(pattern_matches_pub("create a * file", "create a test file"));
        assert!(!pattern_matches_pub("create a * file", "delete a test file"));
    }

    #[test]
    fn test_skill_metadata_default() {
        let meta = SkillMetadata::default();
        assert!(meta.author.is_empty());
        assert!(!meta.cacheable);
        assert!(!meta.idempotent);
        assert!(!meta.reversible);
        assert!(meta.reverse_skill.is_none());
    }

    #[test]
    fn test_sandbox_level_ordering() {
        assert!(SandboxLevel::None < SandboxLevel::Basic);
        assert!(SandboxLevel::Basic < SandboxLevel::Strict);
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_param_type_serde() {
        for pt in [ParamType::String, ParamType::Number, ParamType::Boolean, ParamType::Array, ParamType::Object, ParamType::Path] {
            let json = serde_json::to_string(&pt).unwrap();
            let restored: ParamType = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, pt);
        }
    }

    #[test]
    fn test_skill_source_serde() {
        let sources = vec![
            SkillSource::Builtin,
            SkillSource::User,
            SkillSource::OpenClaw,
            SkillSource::Mcp { server: "test".into() },
        ];
        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let restored: SkillSource = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, source);
        }
    }

    #[test]
    fn test_requirement_serde() {
        let reqs = vec![
            Requirement::Sister("memory".into()),
            Requirement::Permission("admin".into()),
            Requirement::Network,
            Requirement::FileSystem,
            Requirement::Environment("API_KEY".into()),
        ];
        for req in reqs {
            let json = serde_json::to_string(&req).unwrap();
            let restored: Requirement = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, req);
        }
    }

    #[test]
    fn test_constraint_max_length_validation() {
        let skill = SkillDefinition {
            id: "s1".into(),
            name: "test".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            triggers: vec![SkillTrigger::Intent("test".into())],
            parameters: vec![SkillParam {
                name: "text".into(),
                param_type: ParamType::String,
                required: true,
                description: "text".into(),
                default: None,
                constraints: vec![Constraint::MinLength(3)],
            }],
            outputs: vec![],
            requirements: vec![],
            source: SkillSource::Builtin,
            sandbox_level: SandboxLevel::None,
            risk_level: RiskLevel::Low,
            metadata: SkillMetadata::default(),
        };
        let short = HashMap::from([("text".into(), serde_json::json!("ab"))]);
        assert!(skill.validate_inputs(&short).is_err());
        let ok = HashMap::from([("text".into(), serde_json::json!("abc"))]);
        assert!(skill.validate_inputs(&ok).is_ok());
    }

    #[test]
    fn test_constraint_one_of() {
        let skill = SkillDefinition {
            id: "s2".into(),
            name: "deploy".into(),
            version: "1.0.0".into(),
            description: "deploy".into(),
            triggers: vec![SkillTrigger::Intent("deploy".into())],
            parameters: vec![SkillParam {
                name: "env".into(),
                param_type: ParamType::String,
                required: true,
                description: "env".into(),
                default: None,
                constraints: vec![Constraint::OneOf(vec![serde_json::json!("prod"), serde_json::json!("staging")])],
            }],
            outputs: vec![],
            requirements: vec![],
            source: SkillSource::Builtin,
            sandbox_level: SandboxLevel::None,
            risk_level: RiskLevel::Low,
            metadata: SkillMetadata::default(),
        };
        let valid = HashMap::from([("env".into(), serde_json::json!("prod"))]);
        assert!(skill.validate_inputs(&valid).is_ok());
        let invalid = HashMap::from([("env".into(), serde_json::json!("dev"))]);
        assert!(skill.validate_inputs(&invalid).is_err());
    }

    #[test]
    fn test_no_network_requirement() {
        let mut skill = test_skill();
        skill.requirements = vec![Requirement::FileSystem];
        assert!(!skill.needs_network());
    }

    #[test]
    fn test_trigger_serde() {
        let triggers = vec![
            SkillTrigger::Pattern("create * file".into()),
            SkillTrigger::Intent("create".into()),
            SkillTrigger::Tool("fs.create".into()),
        ];
        for trigger in triggers {
            let json = serde_json::to_string(&trigger).unwrap();
            let restored: SkillTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, trigger);
        }
    }
}
