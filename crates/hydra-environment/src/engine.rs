//! EnvironmentEngine — unified coordinator.

use crate::{
    checker::{check_requirements, CheckOutcome},
    constants::MAX_SKILL_ENVIRONMENTS,
    detector::EnvironmentDetector,
    errors::EnvironmentError,
    profile::EnvironmentProfile,
    requirements::SkillRequirements,
};
use std::collections::HashMap;

/// The environment engine manages environment probing and skill compatibility.
pub struct EnvironmentEngine {
    detector: EnvironmentDetector,
    /// The detected environment profile, if probed.
    pub current: Option<EnvironmentProfile>,
    /// Registered skill requirements by skill name.
    requirements: HashMap<String, SkillRequirements>,
}

impl EnvironmentEngine {
    /// Create a new environment engine with no profile or requirements.
    pub fn new() -> Self {
        Self {
            detector: EnvironmentDetector::new(),
            current: None,
            requirements: HashMap::new(),
        }
    }

    /// Probe and cache the current environment.
    pub fn probe(&mut self) -> Result<&EnvironmentProfile, EnvironmentError> {
        let profile = self.detector.detect_current()?;
        self.current = Some(profile);
        // Safe: we just set it on the line above
        Ok(self.current.as_ref().expect("profile was just set"))
    }

    /// Register a skill's environment requirements (called on skill load).
    pub fn register_requirements(&mut self, reqs: SkillRequirements) {
        if self.requirements.len() < MAX_SKILL_ENVIRONMENTS {
            self.requirements.insert(reqs.skill_name.clone(), reqs);
        }
    }

    /// Unregister a skill's requirements (called on skill unload).
    pub fn unregister_skill(&mut self, skill: &str) {
        self.requirements.remove(skill);
    }

    /// Check if a skill can run in the current environment.
    pub fn check_skill(&mut self, skill: &str) -> Result<CheckOutcome, EnvironmentError> {
        let profile = match &self.current {
            Some(p) => p.clone(),
            None => self.detector.detect_current()?,
        };

        let reqs =
            self.requirements
                .get(skill)
                .ok_or_else(|| EnvironmentError::RequirementsNotRegistered {
                    skill: skill.to_string(),
                })?;

        Ok(check_requirements(reqs, &profile))
    }

    /// Check if a specific binary is available.
    pub fn has_binary(&self, binary: &str) -> bool {
        self.current
            .as_ref()
            .map(|p| p.has_binary(binary))
            .unwrap_or_else(|| self.detector.binary_exists(binary))
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        match &self.current {
            Some(p) => format!(
                "env: {} | ram={}MB | os={} | binaries={}",
                p.class.label(),
                p.capabilities.ram_mb,
                p.capabilities.os_type.label(),
                p.available_binaries.len(),
            ),
            None => "env: not yet probed".into(),
        }
    }

    /// Number of registered skill requirements.
    pub fn registered_count(&self) -> usize {
        self.requirements.len()
    }
}

impl Default for EnvironmentEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_succeeds() {
        let mut engine = EnvironmentEngine::new();
        let profile = engine.probe().expect("probe should succeed");
        assert!(!profile.id.is_empty());
        assert!(profile.capabilities.cpu_cores > 0);
    }

    #[test]
    fn summary_after_probe() {
        let mut engine = EnvironmentEngine::new();
        engine.probe().expect("probe should succeed");
        let s = engine.summary();
        assert!(s.contains("env:"));
        assert!(s.contains("ram="));
    }

    #[test]
    fn unknown_skill_returns_error() {
        let mut engine = EnvironmentEngine::new();
        let r = engine.check_skill("nonexistent-skill");
        assert!(matches!(
            r,
            Err(EnvironmentError::RequirementsNotRegistered { .. })
        ));
    }

    #[test]
    fn register_and_check() {
        let mut engine = EnvironmentEngine::new();
        let reqs = SkillRequirements::new("hello-skill");
        engine.register_requirements(reqs);
        engine.probe().expect("probe should succeed");
        let outcome = engine.check_skill("hello-skill").expect("check failed");
        assert!(outcome.can_execute());
    }
}
