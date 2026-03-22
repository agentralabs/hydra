//! The immune system — evaluates threats and generates antibodies.

use crate::antibody::Antibody;
use crate::antifragile::AntifragileStore;
use crate::constants::*;
use crate::errors::AdversaryError;
use crate::threat::{ThreatClass, ThreatSignal};
use serde::{Deserialize, Serialize};

/// Action taken by the immune system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImmuneAction {
    /// Signal is clean, pass through.
    PassThrough,
    /// Signal was blocked by an existing antibody.
    Blocked,
    /// A new antibody was generated for a novel threat.
    NewAntibodyGenerated,
    /// Signal is being monitored (low confidence match).
    Monitoring,
}

/// The immune system's response to a threat signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneResponse {
    /// Action taken.
    pub action: ImmuneAction,
    /// Threat class of the signal.
    pub threat_class: ThreatClass,
    /// Severity of the threat.
    pub severity: f64,
    /// Whether a constitutional threat was detected.
    pub constitutional: bool,
    /// Description of the response.
    pub description: String,
}

/// The immune system managing antibodies and antifragile resistance.
#[derive(Debug, Clone)]
pub struct ImmuneSystem {
    /// All antibodies (never deleted).
    antibodies: Vec<Antibody>,
    /// Antifragile resistance store.
    pub antifragile: AntifragileStore,
}

impl ImmuneSystem {
    /// Create a new immune system.
    pub fn new() -> Self {
        Self {
            antibodies: Vec::new(),
            antifragile: AntifragileStore::new(),
        }
    }

    /// Evaluate a threat signal and return the immune response.
    pub fn evaluate(&mut self, signal: &ThreatSignal) -> Result<ImmuneResponse, AdversaryError> {
        let severity = signal.class.severity();
        let constitutional = signal.class.is_constitutional();

        // Check if any existing antibody recognizes this threat
        let mut matched_idx = None;
        for (i, ab) in self.antibodies.iter().enumerate() {
            if ab.target_class == signal.class && ab.recognizes(&signal.features) {
                matched_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = matched_idx {
            // Existing antibody recognized the threat — block it
            self.antibodies[idx].record_trigger();
            self.antifragile.record_encounter(signal.class, true);

            let response = ImmuneResponse {
                action: ImmuneAction::Blocked,
                threat_class: signal.class,
                severity,
                constitutional,
                description: format!(
                    "Blocked by antibody {} (confidence: {:.2})",
                    self.antibodies[idx].id, self.antibodies[idx].confidence
                ),
            };

            if constitutional {
                return Err(AdversaryError::ConstitutionalThreat {
                    description: response.description.clone(),
                });
            }

            return Ok(response);
        }

        // No antibody match — is this a known threat class?
        if severity > 0.0 && signal.class != ThreatClass::Unknown {
            // Generate a new antibody
            if self.antibodies.len() >= MAX_ANTIBODIES {
                return Err(AdversaryError::AntibodyCapacity {
                    current: self.antibodies.len(),
                    max: MAX_ANTIBODIES,
                });
            }

            let new_ab = Antibody::new(signal.class, signal.features.clone());
            self.antibodies.push(new_ab);
            self.antifragile.record_encounter(signal.class, false);

            let response = ImmuneResponse {
                action: ImmuneAction::NewAntibodyGenerated,
                threat_class: signal.class,
                severity,
                constitutional,
                description: format!("New antibody generated for {}", signal.class.label()),
            };

            if constitutional {
                return Err(AdversaryError::ConstitutionalThreat {
                    description: response.description.clone(),
                });
            }

            return Ok(response);
        }

        // Clean signal
        Ok(ImmuneResponse {
            action: ImmuneAction::PassThrough,
            threat_class: signal.class,
            severity,
            constitutional: false,
            description: "Signal is clean".to_string(),
        })
    }

    /// Return the number of antibodies.
    pub fn antibody_count(&self) -> usize {
        self.antibodies.len()
    }

    /// Return all antibodies (read-only).
    pub fn antibodies(&self) -> &[Antibody] {
        &self.antibodies
    }
}

impl Default for ImmuneSystem {
    fn default() -> Self {
        Self::new()
    }
}
