//! HydraTime — temporal memory with time-aware recall.
//!
//! Stores memories with temporal metadata and supports
//! time-range queries, recency weighting, and temporal patterns.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A time range for querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

/// A temporal memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub timestamp: String,
    pub epoch_ms: i64,
    pub importance: f64,
    pub access_count: u64,
    pub last_accessed: String,
    pub decay_rate: f64,
}

impl TemporalEntry {
    pub fn new(content: &str, category: &str, importance: f64) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: content.into(),
            category: category.into(),
            timestamp: now.to_rfc3339(),
            epoch_ms: now.timestamp_millis(),
            importance: importance.clamp(0.0, 1.0),
            access_count: 0,
            last_accessed: now.to_rfc3339(),
            decay_rate: 0.01,
        }
    }

    /// Calculate effective importance (decays with time since last access)
    pub fn effective_importance(&self, now_epoch_ms: i64) -> f64 {
        let age_hours = (now_epoch_ms - self.epoch_ms).max(0) as f64 / 3_600_000.0;
        let decay = (-self.decay_rate * age_hours).exp();
        let access_boost = (self.access_count as f64).ln().max(0.0) * 0.1;
        (self.importance * decay + access_boost).clamp(0.0, 1.0)
    }

    /// Mark as accessed
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = chrono::Utc::now().to_rfc3339();
    }
}

/// Temporal query parameters
#[derive(Debug, Clone)]
pub struct TemporalQuery {
    pub keyword: Option<String>,
    pub category: Option<String>,
    pub time_range: Option<TimeRange>,
    pub min_importance: Option<f64>,
    pub limit: usize,
}

impl Default for TemporalQuery {
    fn default() -> Self {
        Self {
            keyword: None,
            category: None,
            time_range: None,
            min_importance: None,
            limit: 10,
        }
    }
}

/// Temporal memory store
pub struct HydraTime {
    entries: parking_lot::RwLock<Vec<TemporalEntry>>,
    category_index: parking_lot::RwLock<HashMap<String, Vec<String>>>,
    max_entries: usize,
}

impl HydraTime {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: parking_lot::RwLock::new(Vec::new()),
            category_index: parking_lot::RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    /// Store a temporal memory
    pub fn store(&self, content: &str, category: &str, importance: f64) -> String {
        let entry = TemporalEntry::new(content, category, importance);
        let id = entry.id.clone();

        self.category_index
            .write()
            .entry(category.into())
            .or_default()
            .push(id.clone());

        let mut entries = self.entries.write();
        entries.push(entry);

        // Evict least important if over limit
        if entries.len() > self.max_entries {
            let now = chrono::Utc::now().timestamp_millis();
            entries.sort_by(|a, b| {
                b.effective_importance(now)
                    .partial_cmp(&a.effective_importance(now))
                    .unwrap()
            });
            entries.truncate(self.max_entries);
        }

        id
    }

    /// Recall memories matching a query, ranked by effective importance
    pub fn recall(&self, query: &TemporalQuery) -> Vec<TemporalEntry> {
        let now = chrono::Utc::now().timestamp_millis();
        let entries = self.entries.read();

        let mut results: Vec<TemporalEntry> = entries
            .iter()
            .filter(|e| {
                // Keyword filter
                if let Some(kw) = &query.keyword {
                    if !e.content.to_lowercase().contains(&kw.to_lowercase()) {
                        return false;
                    }
                }
                // Category filter
                if let Some(cat) = &query.category {
                    if &e.category != cat {
                        return false;
                    }
                }
                // Time range filter
                if let Some(range) = &query.time_range {
                    if e.timestamp < range.start || e.timestamp > range.end {
                        return false;
                    }
                }
                // Importance filter
                if let Some(min) = query.min_importance {
                    if e.effective_importance(now) < min {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by effective importance (descending)
        results.sort_by(|a, b| {
            b.effective_importance(now)
                .partial_cmp(&a.effective_importance(now))
                .unwrap()
        });
        results.truncate(query.limit);

        // Mark as accessed
        drop(entries);
        let mut entries = self.entries.write();
        for result in &results {
            if let Some(e) = entries.iter_mut().find(|e| e.id == result.id) {
                e.access();
            }
        }

        results
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<String> {
        self.category_index.read().keys().cloned().collect()
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }

    /// Get entries in a category
    pub fn category_count(&self, category: &str) -> usize {
        self.category_index
            .read()
            .get(category)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_store_recall() {
        let time = HydraTime::new(100);
        time.store("First meeting with user", "events", 0.9);
        time.store("Installed rust toolchain", "actions", 0.5);
        time.store("User prefers dark mode", "preferences", 0.7);

        let query = TemporalQuery {
            keyword: Some("user".into()),
            ..Default::default()
        };
        let results = time.recall(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_category_filtering() {
        let time = HydraTime::new(100);
        time.store("event 1", "events", 0.5);
        time.store("event 2", "events", 0.6);
        time.store("action 1", "actions", 0.7);

        let query = TemporalQuery {
            category: Some("events".into()),
            ..Default::default()
        };
        let results = time.recall(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_importance_decay() {
        let entry = TemporalEntry::new("test", "cat", 0.8);
        let now = entry.epoch_ms;

        // Same time: importance unchanged (no decay)
        let imp_now = entry.effective_importance(now);
        assert!(imp_now > 0.7);

        // Far future: importance decayed
        let far_future = now + 100 * 3_600_000; // 100 hours
        let imp_future = entry.effective_importance(far_future);
        assert!(imp_future < imp_now);
    }

    #[test]
    fn test_importance_ranking() {
        let time = HydraTime::new(100);
        time.store("low importance", "test", 0.1);
        time.store("high importance", "test", 0.95);
        time.store("medium importance", "test", 0.5);

        let query = TemporalQuery::default();
        let results = time.recall(&query);
        assert!(results[0].importance >= results[1].importance);
    }
}
