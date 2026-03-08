use std::collections::HashMap;

use hydra_skills::{
    builtin_skills, McpAdapter, OpenClawAdapter, ParamType, Requirement, RiskLevel, Sandbox,
    SandboxLevel, SandboxOp, SkillDefinition, SkillExecutor, SkillMetadata, SkillParam,
    SkillRegistry, SkillSource, SkillTrigger, SkillValidator,
};

fn make_skill(name: &str, id: &str) -> SkillDefinition {
    SkillDefinition {
        id: id.into(),
        name: name.into(),
        version: "1.0.0".into(),
        description: format!("Skill: {}", name),
        triggers: vec![
            SkillTrigger::Intent(name.into()),
            SkillTrigger::Tool(format!("{}.run", name)),
        ],
        parameters: vec![SkillParam {
            name: "input".into(),
            param_type: ParamType::String,
            required: true,
            description: "Input".into(),
            default: None,
            constraints: vec![],
        }],
        outputs: vec![],
        requirements: vec![],
        source: SkillSource::Builtin,
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata::default(),
    }
}

// === Full pipeline: register → discover → validate → execute ===

#[test]
fn test_full_pipeline() {
    let registry = SkillRegistry::new();

    // Register builtins
    for skill in builtin_skills() {
        registry.register(skill).unwrap();
    }
    assert_eq!(registry.count(), 4);

    // Discover by intent
    let matches = registry.discover("file_read");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "file_read");

    // Look up and validate
    let skill = registry.lookup("file_read").unwrap();
    let validator = SkillValidator::new();
    let validation = validator.validate(&skill);
    assert!(validation.safe);

    // Execute
    let executor = SkillExecutor::new();
    let inputs = HashMap::from([("path".into(), serde_json::json!("/tmp/test.txt"))]);
    let result = executor.execute(&skill, inputs).unwrap();
    assert!(result.success);
    assert_eq!(result.tokens_used, 0);
}

#[test]
fn test_openclaw_import_and_register() {
    let json = r#"{
        "name": "summarize",
        "description": "Summarize text",
        "inputs": {
            "text": { "type": "string", "description": "Text to summarize", "required": true },
            "max_words": { "type": "integer", "description": "Max words", "required": false }
        },
        "outputs": {
            "summary": { "type": "string", "description": "Summary" }
        },
        "tags": ["nlp", "text"]
    }"#;

    let skill = OpenClawAdapter::parse(json).unwrap();
    assert_eq!(skill.source, SkillSource::OpenClaw);

    let registry = SkillRegistry::new();
    registry.register(skill).unwrap();

    let found = registry.discover("summarize");
    assert_eq!(found.len(), 1);
}

#[test]
fn test_mcp_import_and_register() {
    let json = r#"{
        "name": "memory_query",
        "description": "Query memory entries",
        "input_schema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" },
                "limit": { "type": "integer", "description": "Max results" }
            },
            "required": ["query"]
        }
    }"#;

    let skill = McpAdapter::parse("agentic-memory", json).unwrap();
    assert_eq!(
        skill.source,
        SkillSource::Mcp {
            server: "agentic-memory".into()
        }
    );

    let registry = SkillRegistry::new();
    registry.register(skill).unwrap();

    // Discover by tool name
    let found = registry.discover("agentic-memory.memory_query");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].confidence, 1.0);
}

#[test]
fn test_sandbox_isolation_levels() {
    // Strict sandbox blocks everything
    let strict = Sandbox::for_level(SandboxLevel::Strict);
    assert!(!strict.check_operation(&SandboxOp::Network));
    assert!(!strict.check_operation(&SandboxOp::ReadFile("/tmp/x".into())));

    // Basic sandbox allows temp only
    let basic = Sandbox::for_level(SandboxLevel::Basic);
    assert!(basic.check_operation(&SandboxOp::ReadFile("/tmp/x".into())));
    assert!(!basic.check_operation(&SandboxOp::ReadFile("/etc/passwd".into())));
    assert!(basic.check_operation(&SandboxOp::Network));

    // None allows everything
    let none = Sandbox::for_level(SandboxLevel::None);
    assert!(none.check_operation(&SandboxOp::Execute));
}

#[test]
fn test_validator_rejects_unsafe() {
    let validator = SkillValidator::strict();

    // External skill with no sandbox → rejected in strict mode
    let mut skill = make_skill("dangerous", "d1");
    skill.source = SkillSource::OpenClaw;
    skill.sandbox_level = SandboxLevel::None;
    let result = validator.validate(&skill);
    assert!(!result.safe);
}

#[test]
fn test_executor_respects_sandbox() {
    let executor = SkillExecutor::new();

    let mut skill = make_skill("net_skill", "n1");
    skill.sandbox_level = SandboxLevel::Strict;
    skill.requirements.push(Requirement::Network);

    let inputs = HashMap::from([("input".into(), serde_json::json!("test"))]);
    let result = executor.execute(&skill, inputs);
    assert!(result.is_err());
}

#[test]
fn test_registry_list_and_remove() {
    let registry = SkillRegistry::new();
    registry.register(make_skill("a", "s1")).unwrap();
    registry.register(make_skill("b", "s2")).unwrap();

    assert_eq!(registry.list().len(), 2);

    registry.remove("a");
    assert_eq!(registry.count(), 1);
    assert!(registry.lookup("a").is_err());
}

#[test]
fn test_builtin_skills_all_valid() {
    let validator = SkillValidator::new();
    for skill in builtin_skills() {
        let result = validator.validate(&skill);
        assert!(
            result.safe,
            "builtin skill {} is not safe: {:?}",
            skill.name, result.issues
        );
    }
}
