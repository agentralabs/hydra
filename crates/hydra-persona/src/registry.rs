//! Persona registry — manages registered personas and active blends.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::blend::PersonaBlend;
use crate::constants::MAX_PERSONAS;
use crate::errors::PersonaError;
use crate::persona::Persona;
use crate::voice::PersonaVoice;

/// The persona registry — holds all registered personas and the active blend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRegistry {
    /// All registered personas, indexed by name.
    personas: HashMap<String, Persona>,
    /// The currently active voice (if any).
    active_voice: Option<PersonaVoice>,
}

impl PersonaRegistry {
    /// Create a new registry with the core persona pre-loaded.
    pub fn new() -> Self {
        let mut personas = HashMap::new();
        let core = Persona::core_persona();
        personas.insert(core.name.clone(), core);

        Self {
            personas,
            active_voice: None,
        }
    }

    /// Register a new persona.
    pub fn register(&mut self, persona: Persona) -> Result<(), PersonaError> {
        if self.personas.len() >= MAX_PERSONAS {
            return Err(PersonaError::RegistryFull {
                count: self.personas.len(),
                max: MAX_PERSONAS,
            });
        }
        self.personas.insert(persona.name.clone(), persona);
        Ok(())
    }

    /// Look up a persona by name.
    pub fn get(&self, name: &str) -> Option<&Persona> {
        self.personas.get(name)
    }

    /// Return the number of registered personas.
    pub fn count(&self) -> usize {
        self.personas.len()
    }

    /// Set the active blend. All persona names in the blend must be registered.
    pub fn set_blend(&mut self, blend: PersonaBlend) -> Result<(), PersonaError> {
        let personas: Result<Vec<&Persona>, PersonaError> = blend
            .components
            .iter()
            .map(|c| {
                self.personas
                    .get(&c.persona_name)
                    .ok_or_else(|| PersonaError::PersonaNotFound {
                        name: c.persona_name.clone(),
                    })
            })
            .collect();

        let persona_refs: Vec<&Persona> = personas?;
        let voice = PersonaVoice::new(blend, &persona_refs);
        self.active_voice = Some(voice);
        Ok(())
    }

    /// Activate a single persona by name (convenience for single-persona blend).
    pub fn activate(&mut self, name: &str) -> Result<(), PersonaError> {
        if !self.personas.contains_key(name) {
            return Err(PersonaError::PersonaNotFound {
                name: name.to_string(),
            });
        }
        let blend = PersonaBlend::single(name);
        self.set_blend(blend)
    }

    /// Return the currently active voice, if any.
    pub fn active_voice(&self) -> Option<&PersonaVoice> {
        self.active_voice.as_ref()
    }

    /// Return all registered persona names.
    pub fn persona_names(&self) -> Vec<String> {
        self.personas.keys().cloned().collect()
    }
}

impl Default for PersonaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_registry_has_core_persona() {
        let registry = PersonaRegistry::new();
        assert_eq!(registry.count(), 1);
        assert!(registry.get("hydra-core").is_some());
    }

    #[test]
    fn register_and_activate() {
        let mut registry = PersonaRegistry::new();
        registry
            .register(Persona::security_analyst_persona())
            .expect("register");
        registry.activate("security-analyst").expect("activate");
        let voice = registry.active_voice().expect("voice");
        assert!(voice.is_active());
    }

    #[test]
    fn activate_unregistered_fails() {
        let mut registry = PersonaRegistry::new();
        let result = registry.activate("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn blend_with_unregistered_fails() {
        let mut registry = PersonaRegistry::new();
        let blend = PersonaBlend::weighted(vec![
            ("hydra-core".into(), 0.5),
            ("nonexistent".into(), 0.5),
        ])
        .expect("blend");
        let result = registry.set_blend(blend);
        assert!(result.is_err());
    }
}
