use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use hydra_core::types::CompiledIntent;

/// Thread-safe intent cache — Layer 1 of the 4-layer escalation (0 tokens)
pub struct IntentCache {
    entries: DashMap<String, CompiledIntent>,
    hits: AtomicU64,
    misses: AtomicU64,
    max_entries: usize,
}

impl IntentCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            max_entries,
        }
    }

    /// Exact-match cache lookup (0 tokens)
    pub fn get(&self, text: &str) -> Option<CompiledIntent> {
        self.get_with_context(text, None)
    }

    /// Cache lookup with optional context hash
    pub fn get_with_context(&self, text: &str, context: Option<u64>) -> Option<CompiledIntent> {
        let normalized = Self::cache_key(text, context);
        match self.entries.get(&normalized) {
            Some(entry) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.value().clone())
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Store a compiled intent in the cache
    pub fn put(&self, text: &str, intent: CompiledIntent) {
        self.put_with_context(text, None, intent);
    }

    /// Store with optional context hash
    pub fn put_with_context(&self, text: &str, context: Option<u64>, intent: CompiledIntent) {
        if self.entries.len() >= self.max_entries {
            // Evict oldest (simple strategy: remove first entry)
            if let Some(first_key) = self.entries.iter().next().map(|e| e.key().clone()) {
                self.entries.remove(&first_key);
            }
        }
        let key = Self::cache_key(text, context);
        self.entries.insert(key, intent);
    }

    /// Invalidate a cache entry
    pub fn invalidate(&self, text: &str) {
        let key = Self::cache_key(text, None);
        self.entries.remove(&key);
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.entries.clear();
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Cache hit rate (0.0–1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Total cache hits (for TokenMetrics)
    pub fn total_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Build cache key from text and optional context hash
    fn cache_key(text: &str, context: Option<u64>) -> String {
        let normalized = text
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        match context {
            Some(hash) => format!("{normalized}#ctx:{hash}"),
            None => normalized,
        }
    }
}
