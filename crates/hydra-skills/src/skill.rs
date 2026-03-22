//! Skill — one loadable domain capability package.
//! A skill packages everything needed for a domain:
//!   capability nodes, genome entries, persona, functor registrations.

use serde::{Deserialize, Serialize};

/// The domain a skill covers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkillDomain {
    /// Finance and investment.
    Finance,
    /// Security and threat analysis.
    Security,
    /// Software engineering and development.
    SoftwareEngineering,
    /// Data science and analytics.
    DataScience,
    /// Marketing and growth.
    Marketing,
    /// Legal and compliance.
    Legal,
    /// Medicine and healthcare.
    Medicine,
    /// Research and academia.
    Research,
    /// Creative arts and design.
    Creative,
    /// Operations and infrastructure.
    Operations,
    /// Custom domain with a free-form name.
    Custom(String),
}

/// A skill manifest — describes what a skill provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique skill identifier.
    pub id: String,
    /// Human-readable skill name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// The domain this skill belongs to.
    pub domain: SkillDomain,
    /// Human-readable description of the skill.
    pub description: String,
    /// Capability names this skill adds to the self-model.
    pub capabilities: Vec<String>,
    /// Genome entries this skill seeds (proven approaches).
    pub seed_approaches: Vec<String>,
    /// Persona name this skill registers (optional).
    pub persona_name: Option<String>,
    /// Constitutional laws this skill operates under.
    pub required_laws: Vec<u8>,
}

impl SkillManifest {
    /// Create a new skill manifest with required fields.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
        domain: SkillDomain,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            domain,
            description: String::new(),
            capabilities: Vec::new(),
            seed_approaches: Vec::new(),
            persona_name: None,
            required_laws: vec![1, 2, 3, 4, 5, 6, 7],
        }
    }

    /// Set the capabilities for this skill.
    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    /// Set the seed approaches for this skill.
    pub fn with_approaches(mut self, approaches: Vec<String>) -> Self {
        self.seed_approaches = approaches;
        self
    }

    /// Set the persona for this skill.
    pub fn with_persona(mut self, persona: impl Into<String>) -> Self {
        self.persona_name = Some(persona.into());
        self
    }
}

/// A loaded skill instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedSkill {
    /// The skill manifest.
    pub manifest: SkillManifest,
    /// When the skill was loaded.
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    /// Number of times this skill has been used.
    pub use_count: u64,
    /// Whether the skill is currently active.
    pub active: bool,
}

impl LoadedSkill {
    /// Create a new loaded skill from a manifest.
    pub fn new(manifest: SkillManifest) -> Self {
        Self {
            manifest,
            loaded_at: chrono::Utc::now(),
            use_count: 0,
            active: true,
        }
    }

    /// Record a usage of this skill.
    pub fn record_use(&mut self) {
        self.use_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_manifest_created() {
        let m = SkillManifest::new("finance-v1", "Finance", "0.1.0", SkillDomain::Finance)
            .with_capabilities(vec!["risk.assess".into(), "portfolio.optimize".into()])
            .with_approaches(vec!["dcf-valuation".into()]);

        assert_eq!(m.capabilities.len(), 2);
        assert_eq!(m.seed_approaches.len(), 1);
    }

    #[test]
    fn loaded_skill_tracks_use() {
        let m = SkillManifest::new("s1", "Test", "0.1.0", SkillDomain::Research);
        let mut s = LoadedSkill::new(m);
        s.record_use();
        s.record_use();
        assert_eq!(s.use_count, 2);
    }

    #[test]
    fn manifest_has_description() {
        let m = SkillManifest::new("s1", "Test", "0.1.0", SkillDomain::Research);
        assert!(m.description.is_empty());
    }

    #[test]
    fn custom_domain_works() {
        let m = SkillManifest::new(
            "bio-v1",
            "Biotech",
            "0.1.0",
            SkillDomain::Custom("Biotechnology".into()),
        );
        assert_eq!(m.domain, SkillDomain::Custom("Biotechnology".into()));
    }
}
