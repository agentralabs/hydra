//! SkillRegistry — discovery, registration, version management.

use std::collections::HashMap;

use parking_lot::RwLock;
use thiserror::Error;

use crate::definition::{SkillDefinition, SkillId, SkillSource, SkillTrigger};

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("skill not found: {0}")]
    NotFound(String),
    #[error("skill already registered: {0}")]
    AlreadyExists(String),
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("version conflict: {name} v{existing} already registered, got v{new}")]
    VersionConflict {
        name: String,
        existing: String,
        new: String,
    },
}

/// A match result from skill discovery
#[derive(Debug, Clone)]
pub struct SkillMatch {
    pub skill_id: SkillId,
    pub name: String,
    pub confidence: f32,
    pub trigger: SkillTrigger,
}

/// Summary info for listing
#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub id: SkillId,
    pub name: String,
    pub version: String,
    pub source: SkillSource,
    pub trigger_count: usize,
}

/// Central registry for all skills
pub struct SkillRegistry {
    skills: RwLock<HashMap<SkillId, SkillDefinition>>,
    /// name → id index for fast lookup
    name_index: RwLock<HashMap<String, SkillId>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new skill
    pub fn register(&self, skill: SkillDefinition) -> Result<(), RegistryError> {
        // Validate
        if skill.name.is_empty() {
            return Err(RegistryError::ValidationFailed(
                "name cannot be empty".into(),
            ));
        }
        if skill.triggers.is_empty() {
            return Err(RegistryError::ValidationFailed(
                "must have at least one trigger".into(),
            ));
        }

        let mut skills = self.skills.write();
        let mut names = self.name_index.write();

        // Check version conflict
        if let Some(existing_id) = names.get(&skill.name) {
            if let Some(existing) = skills.get(existing_id) {
                if existing.version == skill.version {
                    return Err(RegistryError::VersionConflict {
                        name: skill.name.clone(),
                        existing: existing.version.clone(),
                        new: skill.version.clone(),
                    });
                }
                // Remove old version
                skills.remove(existing_id);
            }
        }

        let id = skill.id.clone();
        names.insert(skill.name.clone(), id.clone());
        skills.insert(id, skill);
        Ok(())
    }

    /// Look up a skill by name
    pub fn lookup(&self, name: &str) -> Result<SkillDefinition, RegistryError> {
        let names = self.name_index.read();
        let id = names
            .get(name)
            .ok_or_else(|| RegistryError::NotFound(name.into()))?;
        let skills = self.skills.read();
        skills
            .get(id)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(name.into()))
    }

    /// Look up a skill by ID
    pub fn get(&self, id: &str) -> Result<SkillDefinition, RegistryError> {
        self.skills
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(id.into()))
    }

    /// Discover skills matching input (intent, pattern, or tool name)
    pub fn discover(&self, input: &str) -> Vec<SkillMatch> {
        let skills = self.skills.read();
        let mut matches = Vec::new();

        for skill in skills.values() {
            for trigger in &skill.triggers {
                let matched = match trigger {
                    SkillTrigger::Pattern(pattern) => {
                        crate::definition::pattern_matches_pub(pattern, input)
                    }
                    SkillTrigger::Intent(intent) => input.eq_ignore_ascii_case(intent),
                    SkillTrigger::Tool(tool) => input == tool,
                };

                if matched {
                    matches.push(SkillMatch {
                        skill_id: skill.id.clone(),
                        name: skill.name.clone(),
                        confidence: match trigger {
                            SkillTrigger::Tool(_) => 1.0,
                            SkillTrigger::Intent(_) => 0.9,
                            SkillTrigger::Pattern(_) => 0.7,
                        },
                        trigger: trigger.clone(),
                    });
                    break; // One match per skill
                }
            }
        }

        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        matches
    }

    /// List all registered skills
    pub fn list(&self) -> Vec<SkillSummary> {
        self.skills
            .read()
            .values()
            .map(|s| SkillSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                version: s.version.clone(),
                source: s.source.clone(),
                trigger_count: s.triggers.len(),
            })
            .collect()
    }

    /// Remove a skill by name
    pub fn remove(&self, name: &str) -> bool {
        let mut names = self.name_index.write();
        if let Some(id) = names.remove(name) {
            self.skills.write().remove(&id);
            true
        } else {
            false
        }
    }

    /// Number of registered skills
    pub fn count(&self) -> usize {
        self.skills.read().len()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::*;

    fn make_skill(name: &str, id: &str) -> SkillDefinition {
        SkillDefinition {
            id: id.into(),
            name: name.into(),
            version: "1.0.0".into(),
            description: format!("Test skill: {}", name),
            triggers: vec![
                SkillTrigger::Intent(name.into()),
                SkillTrigger::Tool(format!("{}.run", name)),
            ],
            parameters: vec![],
            outputs: vec![],
            requirements: vec![],
            source: SkillSource::Builtin,
            sandbox_level: SandboxLevel::None,
            risk_level: RiskLevel::Low,
            metadata: SkillMetadata::default(),
        }
    }

    #[test]
    fn test_registry_register() {
        let registry = SkillRegistry::new();
        assert!(registry.register(make_skill("greet", "s1")).is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_lookup() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let skill = registry.lookup("greet").unwrap();
        assert_eq!(skill.name, "greet");
    }

    #[test]
    fn test_registry_discover() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        registry.register(make_skill("deploy", "s2")).unwrap();

        let matches = registry.discover("greet");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "greet");

        let matches = registry.discover("greet.run");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].confidence, 1.0); // Tool match
    }

    #[test]
    fn test_registry_version_conflict() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let result = registry.register(make_skill("greet", "s2"));
        assert!(matches!(result, Err(RegistryError::VersionConflict { .. })));
    }

    #[test]
    fn test_registry_validation() {
        let registry = SkillRegistry::new();
        let mut skill = make_skill("", "s1");
        skill.name = String::new();
        assert!(matches!(
            registry.register(skill),
            Err(RegistryError::ValidationFailed(_))
        ));

        let mut skill = make_skill("test", "s2");
        skill.triggers.clear();
        assert!(matches!(
            registry.register(skill),
            Err(RegistryError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_registry_default() {
        let registry = SkillRegistry::default();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_get_by_id() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let skill = registry.get("s1").unwrap();
        assert_eq!(skill.name, "greet");
    }

    #[test]
    fn test_registry_get_not_found() {
        let registry = SkillRegistry::new();
        assert!(matches!(registry.get("nope"), Err(RegistryError::NotFound(_))));
    }

    #[test]
    fn test_registry_lookup_not_found() {
        let registry = SkillRegistry::new();
        assert!(matches!(registry.lookup("nope"), Err(RegistryError::NotFound(_))));
    }

    #[test]
    fn test_registry_remove() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        assert!(registry.remove("greet"));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_remove_nonexistent() {
        let registry = SkillRegistry::new();
        assert!(!registry.remove("nope"));
    }

    #[test]
    fn test_registry_list() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("a", "s1")).unwrap();
        registry.register(make_skill("b", "s2")).unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_discover_no_match() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let matches = registry.discover("totally-unrelated");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_discover_intent_confidence() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let matches = registry.discover("greet");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].confidence, 0.9); // Intent match
    }

    #[test]
    fn test_discover_tool_confidence() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let matches = registry.discover("greet.run");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].confidence, 1.0); // Tool match (highest)
    }

    #[test]
    fn test_version_upgrade() {
        let registry = SkillRegistry::new();
        registry.register(make_skill("greet", "s1")).unwrap();
        let mut v2 = make_skill("greet", "s2");
        v2.version = "2.0.0".into();
        registry.register(v2).unwrap();
        let skill = registry.lookup("greet").unwrap();
        assert_eq!(skill.version, "2.0.0");
    }

    #[test]
    fn test_registry_error_display() {
        let err = RegistryError::NotFound("test".into());
        assert!(format!("{}", err).contains("test"));
        let err = RegistryError::AlreadyExists("dup".into());
        assert!(format!("{}", err).contains("dup"));
    }
}
