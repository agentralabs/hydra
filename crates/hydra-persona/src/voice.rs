//! Persona voice — the active communication style.

use serde::{Deserialize, Serialize};

use crate::blend::{BlendedVoice, PersonaBlend};
use crate::persona::Persona;

/// The currently active voice, derived from the active blend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaVoice {
    /// The active blend producing this voice.
    pub blend: PersonaBlend,
    /// The blended voice characteristics.
    pub voice: BlendedVoice,
    /// Whether this voice is currently active.
    pub active: bool,
}

impl PersonaVoice {
    /// Create a new persona voice from a blend and personas.
    pub fn new(blend: PersonaBlend, personas: &[&Persona]) -> Self {
        let voice = blend.blended_voice(personas);
        Self {
            blend,
            voice,
            active: true,
        }
    }

    /// Returns true if this voice is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Deactivate this voice.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Return a human-readable summary of this voice.
    pub fn summary(&self) -> String {
        let dominant = self.blend.dominant().unwrap_or("none");
        let status = if self.active { "active" } else { "inactive" };
        format!(
            "PersonaVoice: dominant={dominant}, tone=\"{tone}\", status={status}",
            tone = self.voice.tone,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_is_active_by_default() {
        let persona = Persona::core_persona();
        let blend = PersonaBlend::single(&persona.name);
        let voice = PersonaVoice::new(blend, &[&persona]);
        assert!(voice.is_active());
    }

    #[test]
    fn voice_can_be_deactivated() {
        let persona = Persona::core_persona();
        let blend = PersonaBlend::single(&persona.name);
        let mut voice = PersonaVoice::new(blend, &[&persona]);
        voice.deactivate();
        assert!(!voice.is_active());
    }

    #[test]
    fn summary_contains_dominant() {
        let persona = Persona::core_persona();
        let blend = PersonaBlend::single(&persona.name);
        let voice = PersonaVoice::new(blend, &[&persona]);
        assert!(voice.summary().contains("hydra-core"));
    }
}
