//! CollectiveLearner — shared learning across Hydra instances.
//!
//! Instances share discoveries, patterns, and corrections so that
//! learning in one instance benefits all others.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Type of learning entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LearningType {
    /// A new pattern was discovered
    PatternDiscovery,
    /// An error was corrected
    ErrorCorrection,
    /// A performance optimization was found
    Optimization,
    /// A new heuristic was validated
    HeuristicValidation,
    /// A safety constraint was learned
    SafetyConstraint,
}

/// A learning entry that can be shared across instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEntry {
    pub id: String,
    pub source_instance: String,
    pub learning_type: LearningType,
    pub description: String,
    pub evidence: serde_json::Value,
    pub confidence: f64,
    pub applicability: f64,
    pub timestamp: String,
    pub applied_count: u64,
}

impl LearningEntry {
    pub fn new(
        source: &str,
        learning_type: LearningType,
        description: &str,
        confidence: f64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_instance: source.into(),
            learning_type,
            description: description.into(),
            evidence: serde_json::json!({}),
            confidence: confidence.clamp(0.0, 1.0),
            applicability: 1.0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            applied_count: 0,
        }
    }

    pub fn with_evidence(mut self, evidence: serde_json::Value) -> Self {
        self.evidence = evidence;
        self
    }

    /// Mark that this learning was applied
    pub fn mark_applied(&mut self) {
        self.applied_count += 1;
    }

    /// Decay confidence over time (not applied = less relevant)
    pub fn decay(&mut self, factor: f64) {
        self.confidence *= factor.clamp(0.0, 1.0);
    }
}

/// Collective learner that aggregates and shares learnings
pub struct CollectiveLearner {
    instance_id: String,
    entries: parking_lot::RwLock<Vec<LearningEntry>>,
    type_counts: parking_lot::RwLock<HashMap<LearningType, usize>>,
    max_entries: usize,
}

impl CollectiveLearner {
    pub fn new(instance_id: &str, max_entries: usize) -> Self {
        Self {
            instance_id: instance_id.into(),
            entries: parking_lot::RwLock::new(Vec::new()),
            type_counts: parking_lot::RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    /// Add a learning from this instance
    pub fn learn(&self, learning_type: LearningType, description: &str, confidence: f64) -> String {
        let entry = LearningEntry::new(&self.instance_id, learning_type, description, confidence);
        let id = entry.id.clone();

        let mut entries = self.entries.write();
        entries.push(entry);

        // Evict oldest if over limit
        while entries.len() > self.max_entries {
            entries.remove(0);
        }

        *self.type_counts.write().entry(learning_type).or_insert(0) += 1;
        id
    }

    /// Import a learning from another instance
    pub fn import(&self, entry: LearningEntry) -> bool {
        // Don't import from self
        if entry.source_instance == self.instance_id {
            return false;
        }

        // Don't import duplicates (by description)
        let entries = self.entries.read();
        if entries.iter().any(|e| e.description == entry.description) {
            return false;
        }
        drop(entries);

        let learning_type = entry.learning_type;
        self.entries.write().push(entry);
        *self.type_counts.write().entry(learning_type).or_insert(0) += 1;
        true
    }

    /// Get learnings relevant to a query (simple keyword match)
    pub fn query(&self, keyword: &str) -> Vec<LearningEntry> {
        let keyword_lower = keyword.to_lowercase();
        self.entries
            .read()
            .iter()
            .filter(|e| e.description.to_lowercase().contains(&keyword_lower))
            .cloned()
            .collect()
    }

    /// Get top learnings by confidence
    pub fn top_learnings(&self, limit: usize) -> Vec<LearningEntry> {
        let mut entries: Vec<_> = self.entries.read().clone();
        entries.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        entries.truncate(limit);
        entries
    }

    /// Get learnings by type
    pub fn by_type(&self, learning_type: LearningType) -> Vec<LearningEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.learning_type == learning_type)
            .cloned()
            .collect()
    }

    /// Export all learnings for sharing with other instances
    pub fn export(&self) -> Vec<LearningEntry> {
        self.entries.read().clone()
    }

    /// Decay all entries by factor
    pub fn decay_all(&self, factor: f64) {
        for entry in self.entries.write().iter_mut() {
            entry.decay(factor);
        }
    }

    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }

    pub fn type_count(&self, learning_type: LearningType) -> usize {
        self.type_counts.read().get(&learning_type).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_learning() {
        let learner = CollectiveLearner::new("instance-1", 100);
        let id = learner.learn(
            LearningType::PatternDiscovery,
            "File edits are faster with batch writes",
            0.85,
        );
        assert!(!id.is_empty());
        assert_eq!(learner.entry_count(), 1);
    }

    #[test]
    fn test_import_from_peer() {
        let learner_a = CollectiveLearner::new("instance-a", 100);
        let learner_b = CollectiveLearner::new("instance-b", 100);

        learner_a.learn(
            LearningType::Optimization,
            "Parallel reads improve throughput",
            0.9,
        );

        let exported = learner_a.export();
        assert_eq!(exported.len(), 1);

        let imported = learner_b.import(exported[0].clone());
        assert!(imported);
        assert_eq!(learner_b.entry_count(), 1);
    }

    #[test]
    fn test_no_self_import() {
        let learner = CollectiveLearner::new("instance-1", 100);
        learner.learn(LearningType::ErrorCorrection, "Fix X", 0.7);

        let exported = learner.export();
        let imported = learner.import(exported[0].clone());
        assert!(!imported); // Can't import own learning
    }

    #[test]
    fn test_query_learnings() {
        let learner = CollectiveLearner::new("inst", 100);
        learner.learn(LearningType::PatternDiscovery, "File writes should be batched", 0.8);
        learner.learn(LearningType::Optimization, "Cache DNS lookups", 0.9);

        let results = learner.query("file");
        assert_eq!(results.len(), 1);
        assert!(results[0].description.contains("File"));
    }

    #[test]
    fn test_decay() {
        let learner = CollectiveLearner::new("inst", 100);
        learner.learn(LearningType::PatternDiscovery, "test", 1.0);

        learner.decay_all(0.5);
        let top = learner.top_learnings(1);
        assert!((top[0].confidence - 0.5).abs() < f64::EPSILON);
    }
}
