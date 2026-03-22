//! Scenario types for oracle projections.

use serde::{Deserialize, Serialize};

/// A single projected scenario with probability and classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name for this scenario.
    pub name: String,
    /// Probability of this scenario occurring (0.0 to 1.0).
    pub probability: f64,
    /// Whether this scenario represents an adverse outcome.
    pub is_adverse: bool,
    /// Description of the scenario.
    pub description: String,
    /// Optional intervention that could prevent or mitigate this scenario.
    pub intervention: Option<String>,
}

impl Scenario {
    /// Create a new scenario with clamped probability.
    pub fn new(
        name: impl Into<String>,
        probability: f64,
        is_adverse: bool,
        description: impl Into<String>,
        intervention: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            probability: probability.clamp(0.0, 1.0),
            is_adverse,
            description: description.into(),
            intervention,
        }
    }

    /// Return a human-readable label for this scenario.
    pub fn label(&self) -> String {
        let kind = if self.is_adverse {
            "ADVERSE"
        } else {
            "POSITIVE"
        };
        format!("[{}] {} (p={:.2})", kind, self.name, self.probability)
    }
}
