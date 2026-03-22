//! Persona definitions — named behavioral profiles for Hydra.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::DEFAULT_PERSONA_NAME;

/// A named behavioral profile that influences Hydra's communication style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of this persona's role.
    pub description: String,
    /// Vocabulary preferences (words this persona favors).
    pub vocabulary: Vec<String>,
    /// Priority domains (what this persona cares about most).
    pub priorities: Vec<String>,
    /// Tone description (e.g., "formal", "terse", "encouraging").
    pub tone: String,
    /// When this persona was created.
    pub created_at: DateTime<Utc>,
}

impl Persona {
    /// Create a new persona with the given name and description.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            vocabulary: Vec::new(),
            priorities: Vec::new(),
            tone: "neutral".to_string(),
            created_at: Utc::now(),
        }
    }

    /// Builder: set vocabulary preferences.
    pub fn with_vocabulary(mut self, vocab: Vec<String>) -> Self {
        self.vocabulary = vocab;
        self
    }

    /// Builder: set priority domains.
    pub fn with_priorities(mut self, priorities: Vec<String>) -> Self {
        self.priorities = priorities;
        self
    }

    /// Builder: set tone.
    pub fn with_tone(mut self, tone: impl Into<String>) -> Self {
        self.tone = tone.into();
        self
    }

    /// Create the core Hydra persona.
    pub fn core_persona() -> Self {
        Self::new(
            DEFAULT_PERSONA_NAME,
            "The default Hydra persona — balanced, helpful, precise",
        )
        .with_vocabulary(vec![
            "constitutional".into(),
            "causal".into(),
            "growth".into(),
            "capability".into(),
        ])
        .with_priorities(vec![
            "correctness".into(),
            "safety".into(),
            "helpfulness".into(),
        ])
        .with_tone("precise and helpful")
    }

    /// Create a security analyst persona.
    pub fn security_analyst_persona() -> Self {
        Self::new("security-analyst", "Security-focused analysis persona")
            .with_vocabulary(vec![
                "threat".into(),
                "vulnerability".into(),
                "mitigation".into(),
                "attack-surface".into(),
            ])
            .with_priorities(vec![
                "security".into(),
                "risk-assessment".into(),
                "defense-in-depth".into(),
            ])
            .with_tone("cautious and thorough")
    }

    /// Create a software architect persona.
    pub fn software_architect_persona() -> Self {
        Self::new("software-architect", "Architecture and design persona")
            .with_vocabulary(vec![
                "abstraction".into(),
                "interface".into(),
                "dependency".into(),
                "modularity".into(),
            ])
            .with_priorities(vec![
                "maintainability".into(),
                "scalability".into(),
                "simplicity".into(),
            ])
            .with_tone("thoughtful and systematic")
    }
}
