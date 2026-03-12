//! Intent classification statistics — tracks how many times each intent category is classified.

use std::collections::HashMap;
use std::sync::Mutex;

/// Thread-safe intent classification counter.
pub struct IntentStats {
    counts: Mutex<HashMap<String, u64>>,
}

impl IntentStats {
    pub fn new() -> Self {
        IntentStats {
            counts: Mutex::new(HashMap::new()),
        }
    }

    /// Increment count for a category.
    pub fn record(&self, category: &str) {
        let mut map = self.counts.lock().unwrap();
        *map.entry(category.to_string()).or_insert(0) += 1;
    }

    /// Return the top-N most common intents, sorted descending.
    pub fn top_n(&self, n: usize) -> Vec<(String, u64)> {
        let map = self.counts.lock().unwrap();
        let mut counts: Vec<(String, u64)> = map.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.into_iter().take(n).collect()
    }

    /// Total number of classifications.
    pub fn total(&self) -> u64 {
        let map = self.counts.lock().unwrap();
        map.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_total() {
        let stats = IntentStats::new();
        stats.record("Greeting");
        stats.record("CodeBuild");
        assert_eq!(stats.total(), 2);
    }

    #[test]
    fn test_top_n() {
        let stats = IntentStats::new();
        stats.record("Greeting");
        stats.record("Greeting");
        stats.record("CodeBuild");
        let top = stats.top_n(1);
        assert_eq!(top, vec![("Greeting".to_string(), 2)]);
    }

    #[test]
    fn test_empty() {
        let stats = IntentStats::new();
        assert_eq!(stats.total(), 0);
        assert!(stats.top_n(5).is_empty());
    }
}
