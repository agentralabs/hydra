//! Threat ecology — rational agent model of adversaries.

use crate::constants::*;
use crate::threat::ThreatClass;
use chrono::{DateTime, Utc};
use hydra_axiom::AxiomPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A rational threat actor with capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatActor {
    /// Human-readable name.
    pub name: String,
    /// Threat classes this actor is capable of.
    pub capabilities: HashSet<ThreatClass>,
    /// Estimated skill level [0.0, 1.0].
    pub skill_level: f64,
    /// When this actor was first observed.
    pub first_seen: DateTime<Utc>,
    /// When this actor was last seen.
    pub last_seen: DateTime<Utc>,
}

impl ThreatActor {
    /// Create a new threat actor.
    pub fn new(name: impl Into<String>, skill_level: f64) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            capabilities: HashSet::new(),
            skill_level: skill_level.clamp(0.0, 1.0),
            first_seen: now,
            last_seen: now,
        }
    }

    /// Add a capability to this actor.
    pub fn add_capability(&mut self, class: ThreatClass) {
        self.capabilities.insert(class);
        self.last_seen = Utc::now();
    }

    /// Return true if this actor is capable of the given threat class.
    pub fn capable_of(&self, class: &ThreatClass) -> bool {
        self.capabilities.contains(class)
    }

    /// Return the highest severity threat this actor can mount.
    pub fn highest_threat(&self) -> f64 {
        self.capabilities
            .iter()
            .map(|c| c.severity())
            .fold(0.0_f64, f64::max)
    }
}

/// The threat ecology — collection of known threat actors.
#[derive(Debug, Clone, Default)]
pub struct ThreatEcology {
    actors: Vec<ThreatActor>,
}

impl ThreatEcology {
    /// Create an empty threat ecology.
    pub fn new() -> Self {
        Self { actors: Vec::new() }
    }

    /// Add a threat actor to the ecology.
    pub fn add_actor(&mut self, actor: ThreatActor) -> Result<(), crate::errors::AdversaryError> {
        if self.actors.len() >= MAX_THREAT_ACTORS {
            return Err(crate::errors::AdversaryError::ThreatActorCapacity {
                current: self.actors.len(),
                max: MAX_THREAT_ACTORS,
            });
        }
        self.actors.push(actor);
        Ok(())
    }

    /// Return all actors capable of a given threat class.
    pub fn capable_of(&self, class: &ThreatClass) -> Vec<&ThreatActor> {
        self.actors.iter().filter(|a| a.capable_of(class)).collect()
    }

    /// Return the highest threat level in the ecology.
    pub fn highest_threat(&self) -> f64 {
        self.actors
            .iter()
            .map(|a| a.highest_threat())
            .fold(0.0_f64, f64::max)
    }

    /// Return the number of actors.
    pub fn actor_count(&self) -> usize {
        self.actors.len()
    }
}

/// Map a threat class to an axiom primitive for cross-domain reasoning.
pub fn to_axiom_primitive(class: &ThreatClass) -> AxiomPrimitive {
    match class {
        ThreatClass::ConstitutionalViolation
        | ThreatClass::CausalChainManipulation
        | ThreatClass::ReceiptTampering => AxiomPrimitive::AdversarialModel,
        ThreatClass::TrustManipulation => AxiomPrimitive::TrustRelation,
        ThreatClass::ResourceExhaustion => AxiomPrimitive::ResourceAllocation,
        ThreatClass::DataExfiltration | ThreatClass::SideChannel => {
            AxiomPrimitive::InformationValue
        }
        _ => AxiomPrimitive::Risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_capabilities() {
        let mut actor = ThreatActor::new("apt1", 0.9);
        actor.add_capability(ThreatClass::PromptInjection);
        assert!(actor.capable_of(&ThreatClass::PromptInjection));
        assert!(!actor.capable_of(&ThreatClass::DataExfiltration));
    }

    #[test]
    fn ecology_highest_threat() {
        let mut ecology = ThreatEcology::new();
        let mut actor = ThreatActor::new("apt1", 0.9);
        actor.add_capability(ThreatClass::ConstitutionalViolation);
        ecology.add_actor(actor).unwrap();
        assert!((ecology.highest_threat() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn axiom_mapping() {
        let p = to_axiom_primitive(&ThreatClass::ConstitutionalViolation);
        assert_eq!(p, AxiomPrimitive::AdversarialModel);
    }
}
