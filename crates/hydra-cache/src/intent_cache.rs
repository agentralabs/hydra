use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use hydra_core::types::CompiledIntent;

/// Entry stored in the cache, wrapping the compiled intent with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    intent: CompiledIntent,
    inserted_at: Instant,
    access_count: u64,
}

/// Thread-safe intent cache with TTL support — Layer 1 of the 4-layer escalation (0 tokens)
pub struct IntentCache {
    entries: DashMap<String, CacheEntry>,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    max_entries: usize,
    ttl: Duration,
}

impl IntentCache {
    /// Create a new cache with a maximum number of entries and a TTL
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            max_entries,
            ttl,
        }
    }

    /// Create a cache with no TTL (entries never expire based on time)
    pub fn without_ttl(max_entries: usize) -> Self {
        Self::new(max_entries, Duration::from_secs(u64::MAX))
    }

    /// Exact-match cache lookup (0 tokens consumed)
    pub fn get(&self, text: &str) -> Option<CompiledIntent> {
        self.get_with_context(text, None)
    }

    /// Cache lookup with optional context hash for disambiguation
    pub fn get_with_context(&self, text: &str, context: Option<u64>) -> Option<CompiledIntent> {
        let key = Self::cache_key(text, context);
        match self.entries.get_mut(&key) {
            Some(mut entry) => {
                // Check TTL
                if entry.inserted_at.elapsed() > self.ttl {
                    drop(entry);
                    self.entries.remove(&key);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    self.evictions.fetch_add(1, Ordering::Relaxed);
                    None
                } else {
                    entry.access_count += 1;
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    Some(entry.intent.clone())
                }
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
        // Evict if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_one();
        }
        let key = Self::cache_key(text, context);
        self.entries.insert(
            key,
            CacheEntry {
                intent,
                inserted_at: Instant::now(),
                access_count: 0,
            },
        );
    }

    /// Invalidate a cache entry
    pub fn invalidate(&self, text: &str) {
        let key = Self::cache_key(text, None);
        self.entries.remove(&key);
    }

    /// Invalidate with context
    pub fn invalidate_with_context(&self, text: &str, context: Option<u64>) {
        let key = Self::cache_key(text, context);
        self.entries.remove(&key);
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        self.entries.clear();
    }

    /// Number of entries currently in the cache
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Cache hit rate (0.0 to 1.0)
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

    /// Total number of cache hits
    pub fn total_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Total number of cache misses
    pub fn total_misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Total number of evictions (TTL or capacity)
    pub fn total_evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Estimated tokens saved by cache hits (each hit saves ~200 tokens of LLM compilation)
    pub fn tokens_saved(&self) -> u64 {
        self.hits.load(Ordering::Relaxed) * 200
    }

    /// Remove expired entries (garbage collection pass)
    pub fn purge_expired(&self) -> usize {
        let mut purged = 0;
        let keys_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.value().inserted_at.elapsed() > self.ttl)
            .map(|entry| entry.key().clone())
            .collect();
        for key in keys_to_remove {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            purged += 1;
        }
        purged
    }

    /// Maximum configured capacity
    pub fn capacity(&self) -> usize {
        self.max_entries
    }

    /// Evict one entry (least-accessed heuristic)
    fn evict_one(&self) {
        // Find the entry with the lowest access count
        let victim = self
            .entries
            .iter()
            .min_by_key(|entry| entry.value().access_count)
            .map(|entry| entry.key().clone());
        if let Some(key) = victim {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Build a normalized cache key from text and optional context hash
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

#[cfg(test)]
#[path = "intent_cache_tests.rs"]
mod tests;
