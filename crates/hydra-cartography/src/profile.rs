//! System profiles for digital cartography.

use crate::system_class::SystemClass;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A profile describing a digital system encountered by Hydra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProfile {
    /// Unique name for this system.
    pub name: String,
    /// Classification of the system.
    pub class: SystemClass,
    /// Hints about the system's interface (e.g., "json", "xml", "protobuf").
    pub interface_hints: BTreeSet<String>,
    /// Known constraints of this system.
    pub constraints: Vec<String>,
    /// Known approaches that work with this system.
    pub approaches: Vec<String>,
    /// Number of times Hydra has encountered this system.
    pub encounter_count: u64,
    /// When this profile was first created.
    pub created_at: DateTime<Utc>,
    /// When this profile was last encountered.
    pub last_seen_at: DateTime<Utc>,
}

impl SystemProfile {
    /// Create a new system profile.
    pub fn new(name: impl Into<String>, class: SystemClass) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            class,
            interface_hints: BTreeSet::new(),
            constraints: Vec::new(),
            approaches: Vec::new(),
            encounter_count: 1,
            created_at: now,
            last_seen_at: now,
        }
    }

    /// Compute similarity between two system profiles.
    ///
    /// Weighted: 70% class similarity + 30% interface hint overlap.
    /// Result is clamped to [0.0, 1.0].
    pub fn similarity(&self, other: &Self) -> f64 {
        let class_sim = self.class.similarity(&other.class);
        let hint_sim = jaccard_similarity(&self.interface_hints, &other.interface_hints);
        let result = 0.7 * class_sim + 0.3 * hint_sim;
        result.clamp(0.0, 1.0)
    }

    /// Record an encounter with this system.
    pub fn record_encounter(&mut self) {
        self.encounter_count += 1;
        self.last_seen_at = Utc::now();
    }

    /// Add a known approach for this system.
    pub fn add_approach(&mut self, approach: impl Into<String>) {
        self.approaches.push(approach.into());
    }

    /// Add an interface hint.
    pub fn add_hint(&mut self, hint: impl Into<String>) {
        self.interface_hints.insert(hint.into());
    }
}

/// Compute Jaccard similarity between two string sets.
fn jaccard_similarity(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_similarity_same_class() {
        let a = SystemProfile::new("api-a", SystemClass::RestApi);
        let b = SystemProfile::new("api-b", SystemClass::RestApi);
        let sim = a.similarity(&b);
        // 0.7*1.0 + 0.3*0.0 = 0.7
        assert!((sim - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn profile_similarity_with_hints() {
        let mut a = SystemProfile::new("api-a", SystemClass::RestApi);
        a.add_hint("json");
        a.add_hint("oauth2");

        let mut b = SystemProfile::new("api-b", SystemClass::RestApi);
        b.add_hint("json");

        let sim = a.similarity(&b);
        // class: 1.0, hints: 1/2 = 0.5 → 0.7*1.0 + 0.3*0.5 = 0.85
        assert!((sim - 0.85).abs() < 0.01);
    }

    #[test]
    fn record_encounter_increments() {
        let mut p = SystemProfile::new("test", SystemClass::CommandLine);
        assert_eq!(p.encounter_count, 1);
        p.record_encounter();
        assert_eq!(p.encounter_count, 2);
    }
}
