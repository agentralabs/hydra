//! SkillValidator — pre-execution safety checks.

use crate::definition::{RiskLevel, SandboxLevel, SkillDefinition, SkillSource};

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub safe: bool,
    pub risk_score: f32,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validates skills before execution
pub struct SkillValidator {
    /// Maximum risk level allowed without approval
    max_auto_risk: RiskLevel,
    /// Whether to allow skills with no sandbox
    allow_no_sandbox: bool,
}

impl SkillValidator {
    pub fn new() -> Self {
        Self {
            max_auto_risk: RiskLevel::Medium,
            allow_no_sandbox: true,
        }
    }

    pub fn strict() -> Self {
        Self {
            max_auto_risk: RiskLevel::Low,
            allow_no_sandbox: false,
        }
    }

    /// Validate a skill definition for safety
    pub fn validate(&self, skill: &SkillDefinition) -> ValidationResult {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        let mut risk_score: f32 = 0.0;

        // Check sandbox level
        if skill.sandbox_level == SandboxLevel::None && !self.allow_no_sandbox {
            issues.push("skill has no sandbox and strict mode is enabled".into());
        }

        // Risk assessment
        match skill.risk_level {
            RiskLevel::Low => risk_score += 0.1,
            RiskLevel::Medium => risk_score += 0.4,
            RiskLevel::High => risk_score += 0.7,
            RiskLevel::Critical => risk_score += 1.0,
        }

        // External skills get higher risk
        match &skill.source {
            SkillSource::Builtin => {}
            SkillSource::User => risk_score += 0.1,
            SkillSource::OpenClaw => risk_score += 0.2,
            SkillSource::Mcp { .. } => risk_score += 0.15,
        }

        // No sandbox + external source = warning
        if skill.sandbox_level == SandboxLevel::None
            && !matches!(skill.source, SkillSource::Builtin)
        {
            warnings.push("external skill with no sandbox".into());
            risk_score += 0.2;
        }

        // Network requirement in strict sandbox
        if skill.needs_network() && skill.sandbox_level == SandboxLevel::Strict {
            issues.push("skill requires network but uses strict sandbox".into());
        }

        // Check risk against threshold
        if skill.risk_level > self.max_auto_risk {
            warnings.push(format!(
                "risk level {:?} exceeds auto-approve threshold {:?}",
                skill.risk_level, self.max_auto_risk
            ));
        }

        // Empty description
        if skill.description.is_empty() {
            warnings.push("skill has no description".into());
        }

        let safe = issues.is_empty();

        ValidationResult {
            safe,
            risk_score: risk_score.min(1.0),
            issues,
            warnings,
        }
    }
}

impl Default for SkillValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::*;

    fn make_skill(source: SkillSource, sandbox: SandboxLevel, risk: RiskLevel) -> SkillDefinition {
        SkillDefinition {
            id: "v-1".into(),
            name: "test".into(),
            version: "1.0.0".into(),
            description: "Test skill".into(),
            triggers: vec![SkillTrigger::Intent("test".into())],
            parameters: vec![],
            outputs: vec![],
            requirements: vec![],
            source,
            sandbox_level: sandbox,
            risk_level: risk,
            metadata: SkillMetadata::default(),
        }
    }

    #[test]
    fn test_validator_safe_builtin() {
        let validator = SkillValidator::new();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::None, RiskLevel::Low);
        let result = validator.validate(&skill);
        assert!(result.safe);
        assert!(result.risk_score < 0.2);
    }

    #[test]
    fn test_validator_external_no_sandbox_warning() {
        let validator = SkillValidator::new();
        let skill = make_skill(SkillSource::OpenClaw, SandboxLevel::None, RiskLevel::Medium);
        let result = validator.validate(&skill);
        assert!(result.safe); // Warning but not blocking in default mode
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validator_strict_rejects_no_sandbox() {
        let validator = SkillValidator::strict();
        let skill = make_skill(SkillSource::User, SandboxLevel::None, RiskLevel::Low);
        let result = validator.validate(&skill);
        assert!(!result.safe);
    }

    #[test]
    fn test_validator_network_sandbox_conflict() {
        let validator = SkillValidator::new();
        let mut skill = make_skill(SkillSource::Builtin, SandboxLevel::Strict, RiskLevel::Low);
        skill.requirements.push(Requirement::Network);
        let result = validator.validate(&skill);
        assert!(!result.safe);
    }

    #[test]
    fn test_validator_default() {
        let v = SkillValidator::default();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::None, RiskLevel::Low);
        assert!(v.validate(&skill).safe);
    }

    #[test]
    fn test_validator_high_risk_warning() {
        let v = SkillValidator::new();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::Basic, RiskLevel::High);
        let result = v.validate(&skill);
        assert!(result.safe); // high risk is a warning, not blocking
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_validator_critical_risk() {
        let v = SkillValidator::new();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::Basic, RiskLevel::Critical);
        let result = v.validate(&skill);
        assert!(result.risk_score > 0.9);
    }

    #[test]
    fn test_validator_mcp_source_risk() {
        let v = SkillValidator::new();
        let skill = make_skill(SkillSource::Mcp { server: "test".into() }, SandboxLevel::Basic, RiskLevel::Low);
        let result = v.validate(&skill);
        assert!(result.risk_score > 0.1); // MCP adds 0.15
    }

    #[test]
    fn test_validator_empty_description_warning() {
        let v = SkillValidator::new();
        let mut skill = make_skill(SkillSource::Builtin, SandboxLevel::None, RiskLevel::Low);
        skill.description = String::new();
        let result = v.validate(&skill);
        assert!(result.warnings.iter().any(|w| w.contains("no description")));
    }

    #[test]
    fn test_validator_strict_low_risk_ok() {
        let v = SkillValidator::strict();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::Basic, RiskLevel::Low);
        let result = v.validate(&skill);
        assert!(result.safe);
    }

    #[test]
    fn test_validation_result_fields() {
        let v = SkillValidator::new();
        let skill = make_skill(SkillSource::Builtin, SandboxLevel::None, RiskLevel::Low);
        let result = v.validate(&skill);
        assert!(result.safe);
        assert!(result.issues.is_empty());
        assert!(result.risk_score >= 0.0);
        assert!(result.risk_score <= 1.0);
    }
}
