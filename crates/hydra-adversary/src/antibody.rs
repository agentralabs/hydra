//! Antibodies — learned threat patterns.

use crate::constants::*;
use crate::threat::ThreatClass;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An antibody: a learned pattern that recognizes threats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Antibody {
    /// Unique identifier.
    pub id: Uuid,
    /// The threat class this antibody targets.
    pub target_class: ThreatClass,
    /// Feature signature this antibody recognizes.
    pub signature: Vec<f64>,
    /// Confidence level (grows with successful defenses).
    pub confidence: f64,
    /// Number of times this antibody has been triggered.
    pub trigger_count: u64,
    /// When this antibody was created.
    pub created_at: DateTime<Utc>,
    /// Last time this antibody triggered.
    pub last_triggered: Option<DateTime<Utc>>,
}

impl Antibody {
    /// Create a new antibody targeting a specific threat class.
    pub fn new(target_class: ThreatClass, signature: Vec<f64>) -> Self {
        Self {
            id: Uuid::new_v4(),
            target_class,
            signature,
            confidence: 0.5,
            trigger_count: 0,
            created_at: Utc::now(),
            last_triggered: None,
        }
    }

    /// Compute recognition strength against a feature vector (cosine similarity).
    pub fn recognition_strength(&self, features: &[f64]) -> f64 {
        cosine_similarity(&self.signature, features)
    }

    /// Return true if this antibody recognizes the given features.
    pub fn recognizes(&self, features: &[f64]) -> bool {
        self.recognition_strength(features) >= ANTIBODY_RECOGNITION_THRESHOLD
    }

    /// Record that this antibody was triggered.
    pub fn record_trigger(&mut self) {
        self.trigger_count += 1;
        self.last_triggered = Some(Utc::now());
        self.confidence = (self.confidence + ANTIBODY_CONFIDENCE_BOOST).min(1.0);
    }

    /// Return the effectiveness of this antibody (confidence * trigger history).
    pub fn effectiveness(&self) -> f64 {
        if self.trigger_count == 0 {
            return self.confidence;
        }
        self.confidence * (1.0 + (self.trigger_count as f64).ln())
    }
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let len = a.len().min(b.len());
    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        return 0.0;
    }

    dot / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let a = vec![1.0, 0.0, 1.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn antibody_recognizes_similar() {
        let sig = vec![1.0, 0.0, 1.0];
        let ab = Antibody::new(ThreatClass::PromptInjection, sig.clone());
        assert!(ab.recognizes(&sig));
    }
}
