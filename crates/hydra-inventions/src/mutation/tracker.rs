//! PatternTracker — track action pattern success rates.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A tracked action pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternRecord {
    pub id: String,
    pub name: String,
    pub actions: Vec<String>,
    pub total_executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_duration_ms: f64,
    pub last_used: String,
}

impl PatternRecord {
    pub fn new(name: &str, actions: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            actions,
            total_executions: 0,
            successes: 0,
            failures: 0,
            avg_duration_ms: 0.0,
            last_used: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.successes as f64 / self.total_executions as f64
    }

    pub fn record_execution(&mut self, success: bool, duration_ms: f64) {
        self.total_executions += 1;
        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }
        // Running average
        self.avg_duration_ms = (self.avg_duration_ms * (self.total_executions - 1) as f64
            + duration_ms)
            / self.total_executions as f64;
        self.last_used = chrono::Utc::now().to_rfc3339();
    }
}

/// Tracks patterns and their performance
pub struct PatternTracker {
    patterns: parking_lot::RwLock<HashMap<String, PatternRecord>>,
}

impl PatternTracker {
    pub fn new() -> Self {
        Self {
            patterns: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Register a new pattern
    pub fn register(&self, name: &str, actions: Vec<String>) -> String {
        let record = PatternRecord::new(name, actions);
        let id = record.id.clone();
        self.patterns.write().insert(id.clone(), record);
        id
    }

    /// Record an execution result
    pub fn record(&self, pattern_id: &str, success: bool, duration_ms: f64) -> bool {
        if let Some(record) = self.patterns.write().get_mut(pattern_id) {
            record.record_execution(success, duration_ms);
            true
        } else {
            false
        }
    }

    /// Get pattern by ID
    pub fn get(&self, pattern_id: &str) -> Option<PatternRecord> {
        self.patterns.read().get(pattern_id).cloned()
    }

    /// Get patterns sorted by success rate (descending)
    pub fn top_patterns(&self, limit: usize) -> Vec<PatternRecord> {
        let mut patterns: Vec<_> = self.patterns.read().values().cloned().collect();
        patterns.sort_by(|a, b| b.success_rate().partial_cmp(&a.success_rate()).unwrap());
        patterns.truncate(limit);
        patterns
    }

    /// Get underperforming patterns (below threshold)
    pub fn underperforming(&self, min_rate: f64, min_executions: u64) -> Vec<PatternRecord> {
        self.patterns
            .read()
            .values()
            .filter(|p| p.total_executions >= min_executions && p.success_rate() < min_rate)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.patterns.read().len()
    }
}

impl Default for PatternTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_tracking() {
        let tracker = PatternTracker::new();
        let id = tracker.register(
            "file_edit",
            vec!["read".into(), "modify".into(), "write".into()],
        );

        tracker.record(&id, true, 100.0);
        tracker.record(&id, true, 150.0);
        tracker.record(&id, false, 200.0);

        let pattern = tracker.get(&id).unwrap();
        assert_eq!(pattern.total_executions, 3);
        assert_eq!(pattern.successes, 2);
    }

    #[test]
    fn test_success_rate() {
        let mut record = PatternRecord::new("test", vec![]);
        record.record_execution(true, 10.0);
        record.record_execution(true, 20.0);
        record.record_execution(false, 30.0);
        record.record_execution(false, 40.0);
        assert!((record.success_rate() - 0.5).abs() < f64::EPSILON);
    }
}
