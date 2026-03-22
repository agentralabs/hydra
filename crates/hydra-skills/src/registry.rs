//! SkillRegistry — all currently loaded skills.

use crate::{
    constants::MAX_LOADED_SKILLS,
    errors::SkillError,
    gate::SkillGate,
    skill::{LoadedSkill, SkillManifest},
};
use hydra_constitution::constants::CONSTITUTIONAL_IDENTITY_ID;
use std::collections::HashMap;

/// The skill registry — all loaded skills.
pub struct SkillRegistry {
    /// Active skills, indexed by skill ID.
    skills: HashMap<String, LoadedSkill>,
    /// The constitutional gate for load checks.
    gate: SkillGate,
    /// Skills ever loaded (includes unloaded — knowledge persists).
    ever_loaded: Vec<String>,
}

impl SkillRegistry {
    /// Create an empty skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            gate: SkillGate::new(),
            ever_loaded: Vec::new(),
        }
    }

    /// Load a skill. Constitutional check first.
    pub fn load(&mut self, manifest: SkillManifest) -> Result<(), SkillError> {
        if self.skills.len() >= MAX_LOADED_SKILLS {
            return Err(SkillError::RegistryFull {
                max: MAX_LOADED_SKILLS,
            });
        }

        if self.skills.contains_key(&manifest.id) {
            return Err(SkillError::AlreadyLoaded {
                name: manifest.name.clone(),
            });
        }

        // Constitutional gate
        self.gate
            .check_load(&manifest, CONSTITUTIONAL_IDENTITY_ID)?;

        let id = manifest.id.clone();
        if !self.ever_loaded.contains(&id) {
            self.ever_loaded.push(id.clone());
        }

        self.skills.insert(id, LoadedSkill::new(manifest));
        Ok(())
    }

    /// Unload a skill. Knowledge seeded into genome persists.
    ///
    /// Returns the manifest for genome seeding.
    pub fn unload(&mut self, skill_id: &str) -> Result<SkillManifest, SkillError> {
        let skill = self.skills.remove(skill_id).ok_or(SkillError::NotFound {
            name: skill_id.to_string(),
        })?;
        Ok(skill.manifest)
    }

    /// Get a reference to a loaded skill by ID.
    pub fn get(&self, id: &str) -> Option<&LoadedSkill> {
        self.skills.get(id)
    }

    /// Get a mutable reference to a loaded skill by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut LoadedSkill> {
        self.skills.get_mut(id)
    }

    /// Return the number of currently active (loaded) skills.
    pub fn loaded_count(&self) -> usize {
        self.skills.len()
    }

    /// Total skills ever loaded (only grows — knowledge persists).
    pub fn ever_loaded_count(&self) -> usize {
        self.ever_loaded.len()
    }

    /// Return all currently loaded skills.
    pub fn all(&self) -> Vec<&LoadedSkill> {
        self.skills.values().collect()
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
    use crate::skill::SkillDomain;

    fn manifest(id: &str) -> SkillManifest {
        SkillManifest::new(id, id, "0.1.0", SkillDomain::Research)
    }

    #[test]
    fn load_and_retrieve() {
        let mut reg = SkillRegistry::new();
        reg.load(manifest("finance-v1")).unwrap();
        assert_eq!(reg.loaded_count(), 1);
        assert!(reg.get("finance-v1").is_some());
    }

    #[test]
    fn unload_removes_from_active() {
        let mut reg = SkillRegistry::new();
        reg.load(manifest("s1")).unwrap();
        reg.unload("s1").unwrap();
        assert_eq!(reg.loaded_count(), 0);
    }

    #[test]
    fn ever_loaded_count_persists_after_unload() {
        let mut reg = SkillRegistry::new();
        reg.load(manifest("s1")).unwrap();
        reg.load(manifest("s2")).unwrap();
        reg.unload("s1").unwrap();
        assert_eq!(reg.ever_loaded_count(), 2);
        assert_eq!(reg.loaded_count(), 1);
    }

    #[test]
    fn duplicate_load_rejected() {
        let mut reg = SkillRegistry::new();
        reg.load(manifest("s1")).unwrap();
        assert!(reg.load(manifest("s1")).is_err());
    }
}
