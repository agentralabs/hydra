use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefCategory {
    Preference,
    Fact,
    Convention,
    Correction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefSource {
    UserStated,
    Inferred,
    Corrected,
}

/// A belief Hydra holds about the user, project, or world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub id: Uuid,
    pub category: BeliefCategory,
    pub subject: String,
    pub content: String,
    pub confidence: f32,
    pub source: BeliefSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub confirmations: u32,
    pub contradictions: u32,
    pub active: bool,
    pub supersedes: Option<Uuid>,
    pub superseded_by: Option<Uuid>,
}

impl Belief {
    pub fn new(
        category: BeliefCategory,
        subject: impl Into<String>,
        content: impl Into<String>,
        source: BeliefSource,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            category,
            subject: subject.into(),
            content: content.into(),
            confidence: match source {
                BeliefSource::UserStated => 0.95,
                BeliefSource::Corrected => 0.99,
                BeliefSource::Inferred => 0.6,
            },
            source,
            created_at: now,
            updated_at: now,
            confirmations: 0,
            contradictions: 0,
            active: true,
            supersedes: None,
            superseded_by: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_supersedes(mut self, old_id: Uuid) -> Self {
        self.supersedes = Some(old_id);
        self
    }

    pub fn confirm(&mut self) {
        self.confirmations += 1;
        self.confidence = (self.confidence + 0.02).min(1.0);
        self.updated_at = Utc::now();
    }

    pub fn contradict(&mut self) {
        self.contradictions += 1;
        self.confidence = (self.confidence - 0.05).max(0.0);
        self.updated_at = Utc::now();
    }

    /// Apply time-based confidence decay
    pub fn apply_decay(&mut self, decay_per_day: f32) {
        let days = (Utc::now() - self.updated_at).num_days().max(0) as f32;
        let decay = decay_per_day * days;
        self.confidence = (self.confidence - decay).max(0.0);
        if self.confidence <= 0.0 {
            self.active = false;
        }
    }

    /// Apply time-based confidence decay with explicit days parameter (for testing)
    pub fn apply_decay_days(&mut self, decay_per_day: f32, days: f32) {
        let decay = decay_per_day * days;
        self.confidence = (self.confidence - decay).max(0.0);
        if self.confidence <= 0.0 {
            self.active = false;
        }
    }

    /// Simple word-overlap similarity between two beliefs' subjects
    pub fn subject_similarity(&self, other: &Belief) -> f32 {
        let self_lower = self.subject.to_lowercase();
        let other_lower = other.subject.to_lowercase();
        let a: std::collections::HashSet<&str> = self_lower.split_whitespace().collect();
        let b: std::collections::HashSet<&str> = other_lower.split_whitespace().collect();
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let intersection = a.intersection(&b).count() as f32;
        let union = a.union(&b).count() as f32;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_belief_user_stated_confidence() {
        let b = Belief::new(BeliefCategory::Fact, "test", "content", BeliefSource::UserStated);
        assert!((b.confidence - 0.95).abs() < f32::EPSILON);
        assert!(b.active);
        assert_eq!(b.confirmations, 0);
        assert_eq!(b.contradictions, 0);
    }

    #[test]
    fn new_belief_corrected_confidence() {
        let b = Belief::new(BeliefCategory::Correction, "test", "content", BeliefSource::Corrected);
        assert!((b.confidence - 0.99).abs() < f32::EPSILON);
    }

    #[test]
    fn new_belief_inferred_confidence() {
        let b = Belief::new(BeliefCategory::Fact, "test", "content", BeliefSource::Inferred);
        assert!((b.confidence - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn with_confidence_clamps_above_one() {
        let b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred)
            .with_confidence(1.5);
        assert!((b.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn with_confidence_clamps_below_zero() {
        let b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred)
            .with_confidence(-0.5);
        assert!((b.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn confirm_increases_confidence() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred);
        let before = b.confidence;
        b.confirm();
        assert_eq!(b.confirmations, 1);
        assert!(b.confidence > before);
        assert!((b.confidence - (before + 0.02)).abs() < f32::EPSILON);
    }

    #[test]
    fn contradict_decreases_confidence() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::UserStated);
        let before = b.confidence;
        b.contradict();
        assert_eq!(b.contradictions, 1);
        assert!(b.confidence < before);
        assert!((b.confidence - (before - 0.05)).abs() < f32::EPSILON);
    }

    #[test]
    fn contradict_does_not_go_below_zero() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred)
            .with_confidence(0.01);
        b.contradict();
        assert!((b.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn confirm_does_not_go_above_one() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Corrected);
        // confidence starts at 0.99
        b.confirm(); // 0.99 + 0.02 = 1.01 -> capped to 1.0
        assert!((b.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_decay_days_reduces_confidence() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::UserStated);
        b.apply_decay_days(0.1, 5.0);
        // 0.95 - (0.1 * 5) = 0.45
        assert!((b.confidence - 0.45).abs() < f32::EPSILON);
        assert!(b.active);
    }

    #[test]
    fn apply_decay_days_deactivates_at_zero() {
        let mut b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred);
        b.apply_decay_days(0.1, 10.0);
        // 0.6 - 1.0 => 0.0
        assert!((b.confidence - 0.0).abs() < f32::EPSILON);
        assert!(!b.active);
    }

    #[test]
    fn subject_similarity_identical() {
        let a = Belief::new(BeliefCategory::Fact, "rust coding style", "a", BeliefSource::Inferred);
        let b = Belief::new(BeliefCategory::Fact, "rust coding style", "b", BeliefSource::Inferred);
        assert!((a.subject_similarity(&b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn subject_similarity_partial() {
        let a = Belief::new(BeliefCategory::Fact, "rust coding style", "a", BeliefSource::Inferred);
        let b = Belief::new(BeliefCategory::Fact, "python coding style", "b", BeliefSource::Inferred);
        let sim = a.subject_similarity(&b);
        assert!(sim > 0.0 && sim < 1.0);
    }

    #[test]
    fn subject_similarity_none() {
        let a = Belief::new(BeliefCategory::Fact, "rust coding style", "a", BeliefSource::Inferred);
        let b = Belief::new(BeliefCategory::Fact, "favorite food", "b", BeliefSource::Inferred);
        assert!((a.subject_similarity(&b) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn subject_similarity_case_insensitive() {
        let a = Belief::new(BeliefCategory::Fact, "Rust Coding", "a", BeliefSource::Inferred);
        let b = Belief::new(BeliefCategory::Fact, "rust coding", "b", BeliefSource::Inferred);
        assert!((a.subject_similarity(&b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn with_supersedes_sets_field() {
        let old_id = Uuid::new_v4();
        let b = Belief::new(BeliefCategory::Fact, "test", "c", BeliefSource::Inferred)
            .with_supersedes(old_id);
        assert_eq!(b.supersedes, Some(old_id));
    }

    #[test]
    fn subject_similarity_both_empty() {
        let a = Belief::new(BeliefCategory::Fact, "", "a", BeliefSource::Inferred);
        let b = Belief::new(BeliefCategory::Fact, "", "b", BeliefSource::Inferred);
        assert!((a.subject_similarity(&b) - 1.0).abs() < f32::EPSILON);
    }
}
