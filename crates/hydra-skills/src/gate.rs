//! SkillGate — constitutional check before any skill executes.
//! Every skill execution is checked against the seven laws.
//! Blocked skills are never run.

use crate::errors::SkillError;
use crate::skill::SkillManifest;
use hydra_constitution::constants::CONSTITUTIONAL_IDENTITY_ID;
use hydra_constitution::{ConstitutionChecker, LawCheckContext};

/// The result of a skill gate check.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// Whether the skill is permitted to execute.
    pub permitted: bool,
    /// Human-readable reason for the decision.
    pub reason: String,
}

/// The constitutional gate for skill execution.
pub struct SkillGate {
    /// The constitution checker.
    checker: ConstitutionChecker,
}

impl SkillGate {
    /// Create a new skill gate with a fresh constitution checker.
    pub fn new() -> Self {
        Self {
            checker: ConstitutionChecker::new(),
        }
    }

    /// Check whether a skill can be loaded and executed.
    pub fn check_load(
        &self,
        manifest: &SkillManifest,
        requester: &str,
    ) -> Result<GateResult, SkillError> {
        let ctx = LawCheckContext::new(requester, "skill.load")
            .with_causal_chain(vec![CONSTITUTIONAL_IDENTITY_ID.to_string()]);

        let result = self.checker.check(&ctx);

        if result.is_permitted() {
            Ok(GateResult {
                permitted: true,
                reason: format!(
                    "Skill '{}' cleared all constitutional checks",
                    manifest.name
                ),
            })
        } else {
            Err(SkillError::ConstitutionallyBlocked {
                name: manifest.name.clone(),
                reason: format!("{:?}", result.first_violation()),
            })
        }
    }
}

impl Default for SkillGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillDomain;

    #[test]
    fn valid_skill_passes_gate() {
        let gate = SkillGate::new();
        let manifest =
            SkillManifest::new("test-skill", "Test Skill", "0.1.0", SkillDomain::Research);
        let result = gate.check_load(&manifest, CONSTITUTIONAL_IDENTITY_ID);
        assert!(result.is_ok());
        assert!(result.unwrap().permitted);
    }
}
