//! Category 1: Unit Gap Fill — hydra-skills edge cases.

use hydra_skills::adapters::mcp::{McpSchema, McpToolDefinition};
use hydra_skills::*;

// === Skill circular dependency (no cycles possible in current design, but test validation) ===

#[test]
fn test_skill_validator_strict_rejects_no_sandbox() {
    let mut skill = SkillDefinition {
        id: "test".into(),
        name: "test".into(),
        description: "test".into(),
        version: "1.0.0".into(),
        source: SkillSource::User,
        triggers: vec![],
        parameters: vec![],
        outputs: vec![],
        requirements: vec![],
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata::default(),
    };
    let validator = SkillValidator::strict();
    let result = validator.validate(&skill);
    assert!(!result.safe);

    // With a sandbox, strict validator accepts it
    skill.sandbox_level = SandboxLevel::Basic;
    let result = validator.validate(&skill);
    assert!(result.safe);
}

// === Sandbox escape attempts ===

#[test]
fn test_sandbox_strict_blocks_network() {
    let sandbox = Sandbox::for_level(SandboxLevel::Strict);
    assert!(!sandbox.allows_network());
    assert!(!sandbox.check_operation(&SandboxOp::Network));
}

#[test]
fn test_sandbox_strict_blocks_writes() {
    let sandbox = Sandbox::for_level(SandboxLevel::Strict);
    assert!(!sandbox.check_operation(&SandboxOp::WriteFile("/some/path".into())));
    assert!(!sandbox.temp_dir_only());
}

#[test]
fn test_sandbox_none_allows_all() {
    let sandbox = Sandbox::for_level(SandboxLevel::None);
    assert!(sandbox.allows_network());
    assert!(sandbox.allows_filesystem());
    assert!(sandbox.check_operation(&SandboxOp::Execute));
}

// === Validator all risk factors ===

#[test]
fn test_validator_high_risk_skill() {
    let skill = SkillDefinition {
        id: "risky".into(),
        name: "risky_skill".into(),
        description: "A risky skill".into(),
        version: "1.0.0".into(),
        source: SkillSource::User,
        triggers: vec![SkillTrigger::Pattern("delete *".into())],
        parameters: vec![],
        outputs: vec![],
        requirements: vec![Requirement::Permission("admin".into())],
        sandbox_level: SandboxLevel::None,
        risk_level: RiskLevel::Critical,
        metadata: SkillMetadata::default(),
    };
    let validator = SkillValidator::new();
    let result = validator.validate(&skill);
    assert!(result.safe); // valid but risky
}

// === Registry operations ===

#[test]
fn test_registry_duplicate_registration() {
    let registry = SkillRegistry::new();
    let skill = SkillDefinition {
        id: "test".into(),
        name: "test_skill".into(),
        description: "test".into(),
        version: "1.0.0".into(),
        source: SkillSource::Builtin,
        triggers: vec![SkillTrigger::Intent("test".into())],
        parameters: vec![],
        outputs: vec![],
        requirements: vec![],
        sandbox_level: SandboxLevel::Basic,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata::default(),
    };
    assert!(registry.register(skill.clone()).is_ok());
    assert!(registry.register(skill).is_err()); // duplicate (same name + version)
}

#[test]
fn test_registry_remove_nonexistent() {
    let registry = SkillRegistry::new();
    assert!(!registry.remove("nonexistent"));
}

#[test]
fn test_registry_discover_no_matches() {
    let registry = SkillRegistry::new();
    let matches = registry.discover("zzzzz_no_match_zzzzz");
    assert!(matches.is_empty());
}

// === Executor ===

#[test]
fn test_executor_missing_required_param() {
    let executor = SkillExecutor::new();
    let skill = SkillDefinition {
        id: "test".into(),
        name: "test_skill".into(),
        description: "test".into(),
        version: "1.0.0".into(),
        source: SkillSource::Builtin,
        triggers: vec![],
        parameters: vec![SkillParam {
            name: "required_param".into(),
            description: "required".into(),
            param_type: ParamType::String,
            required: true,
            default: None,
            constraints: vec![],
        }],
        outputs: vec![],
        requirements: vec![],
        sandbox_level: SandboxLevel::Basic,
        risk_level: RiskLevel::Low,
        metadata: SkillMetadata::default(),
    };
    let result = executor.execute(&skill, std::collections::HashMap::new());
    assert!(result.is_err());
}

// === OpenClaw adapter ===

#[test]
fn test_openclaw_parse_invalid_json() {
    let result = OpenClawAdapter::parse("not json");
    assert!(result.is_err());
}

// === MCP adapter ===

#[test]
fn test_mcp_adapter_from_tool() {
    let tool = McpToolDefinition {
        name: "test_tool".into(),
        description: "A test tool".into(),
        input_schema: Some(McpSchema {
            type_: "object".into(),
            properties: std::collections::HashMap::new(),
            required: vec![],
        }),
    };
    let skill = McpAdapter::from_tool("test-server", tool);
    assert_eq!(
        skill.source,
        SkillSource::Mcp {
            server: "test-server".into()
        }
    );
    assert!(skill.name.contains("test_tool"));
}
