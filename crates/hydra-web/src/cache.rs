//! Search cache — persistent, TTL-aware, semantically searchable.
//! Uses a simple on-disk JSON cache. Semantic matching via stemmed query terms
//! so "rust ownership model" hits a cache entry for "how does rust ownership work".

use std::collections::HashMap;
use std::path::PathBuf;

use crate::constants::*;
use crate::types::{ContentFocus, SearchHit, WebSearchResponse};

/// A cached search result with metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    query: String,
    query_terms: Vec<String>,
    hits: Vec<SearchHit>,
    synthesis: Option<String>,
    stored_at: chrono::DateTime<chrono::Utc>,
    ttl_secs: u64,
}

/// Persistent search cache backed by a JSON file.
pub struct SearchCache {
    entries: HashMap<String, CacheEntry>,
    path: PathBuf,
}

impl SearchCache {
    /// Load cache from disk or create empty.
    pub fn new() -> Self {
        let path = dirs::home_dir()
            .unwrap_or_default()
            .join(".hydra/data/web_cache.json");
        let entries = Self::load_from_disk(&path);
        Self { entries, path }
    }

    /// Check cache for a matching query. Uses semantic (stemmed term) matching.
    pub fn check(&self, query: &str, focus: ContentFocus) -> Option<WebSearchResponse> {
        let query_terms = stem_terms(query);
        let ttl = ttl_for_focus(focus);

        // Try exact key first
        let key = cache_key(query);
        if let Some(entry) = self.entries.get(&key) {
            if !is_stale(entry, ttl) {
                return Some(entry_to_response(entry));
            }
        }

        // Semantic match: find entries where stemmed terms overlap significantly
        let mut best: Option<(&CacheEntry, f64)> = None;
        for entry in self.entries.values() {
            if is_stale(entry, ttl) { continue; }
            let overlap = term_overlap(&query_terms, &entry.query_terms);
            if overlap > 0.6 { // 60% term overlap threshold
                if best.as_ref().map_or(true, |(_, s)| overlap > *s) {
                    best = Some((entry, overlap));
                }
            }
        }

        best.map(|(entry, _)| entry_to_response(entry))
    }

    /// Store search results in cache.
    pub fn store(&mut self, query: &str, hits: &[SearchHit], synthesis: Option<&str>, focus: ContentFocus) {
        let key = cache_key(query);
        let entry = CacheEntry {
            query: query.to_string(),
            query_terms: stem_terms(query),
            hits: hits.to_vec(),
            synthesis: synthesis.map(|s| s.to_string()),
            stored_at: chrono::Utc::now(),
            ttl_secs: ttl_for_focus(focus),
        };
        self.entries.insert(key, entry);
        self.persist();
    }

    /// Invalidate a cache entry.
    pub fn invalidate(&mut self, query: &str) {
        let key = cache_key(query);
        self.entries.remove(&key);
        self.persist();
    }

    /// Evict stale entries.
    pub fn evict_stale(&mut self) {
        let now = chrono::Utc::now();
        self.entries.retain(|_, entry| {
            let age = (now - entry.stored_at).num_seconds() as u64;
            age < entry.ttl_secs * 2 // keep for 2x TTL before evicting
        });
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string(&self.entries) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.path, json) {
                    eprintln!("hydra-web: cache persist failed: {e}");
                }
            }
            Err(e) => eprintln!("hydra-web: cache serialize failed: {e}"),
        }
    }

    fn load_from_disk(path: &PathBuf) -> HashMap<String, CacheEntry> {
        match std::fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }
}

impl Default for SearchCache {
    fn default() -> Self { Self::new() }
}

fn cache_key(query: &str) -> String {
    stem_terms(query).join("_")
}

fn ttl_for_focus(focus: ContentFocus) -> u64 {
    match focus {
        ContentFocus::News => CACHE_TTL_NEWS_SECS,
        ContentFocus::Documentation => CACHE_TTL_DOCS_SECS,
        _ => CACHE_TTL_GENERAL_SECS,
    }
}

fn is_stale(entry: &CacheEntry, ttl: u64) -> bool {
    let age = (chrono::Utc::now() - entry.stored_at).num_seconds() as u64;
    age > ttl
}

fn entry_to_response(entry: &CacheEntry) -> WebSearchResponse {
    WebSearchResponse {
        query: entry.query.clone(),
        hits: entry.hits.clone(),
        synthesis: entry.synthesis.clone(),
        from_cache: true,
        engines_used: vec![crate::types::EngineLabel::GenomeCache],
        duration_ms: 0,
    }
}

/// Compute overlap ratio between two term sets.
fn term_overlap(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    let matches = a.iter().filter(|t| b.contains(t)).count();
    let max_len = a.len().max(b.len());
    matches as f64 / max_len as f64
}

/// Tokenize and stem query terms for semantic cache matching.
/// "how does rust ownership work" → ["rust", "owner", "work"]
/// Stop words removed, suffixes stripped.
fn stem_terms(text: &str) -> Vec<String> {
    let stop_words = ["the", "a", "an", "is", "are", "was", "were", "be", "been",
        "do", "does", "did", "how", "what", "why", "when", "where", "which",
        "who", "in", "on", "at", "to", "for", "of", "with", "by", "from",
        "it", "its", "this", "that", "and", "or", "not", "can", "will", "has", "have"];

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2 && !stop_words.contains(w))
        .map(|w| stem(w))
        .collect()
}

fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    for suffix in &["ship", "ing", "tion", "ment", "ness", "able", "ful", "less", "ous", "ive", "ly", "er", "ed", "es", "s"] {
        if w.len() > suffix.len() + 3 {
            if let Some(stripped) = w.strip_suffix(suffix) {
                return stripped.to_string();
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_terms_removes_stop_words() {
        let terms = stem_terms("how does rust ownership work");
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"owner".to_string()));
        assert!(!terms.contains(&"how".to_string()));
        assert!(!terms.contains(&"does".to_string()));
    }

    #[test]
    fn semantic_cache_hit() {
        // "rust ownership model" and "how does rust ownership work" should match
        let a = stem_terms("rust ownership model");
        let b = stem_terms("how does rust ownership work");
        let overlap = term_overlap(&a, &b);
        assert!(overlap > 0.5, "Expected semantic match, got {overlap}");
    }

    #[test]
    fn no_cache_hit_different_topics() {
        let a = stem_terms("rust ownership");
        let b = stem_terms("python web framework");
        let overlap = term_overlap(&a, &b);
        assert!(overlap < 0.3, "Should not match, got {overlap}");
    }

    #[test]
    fn cache_key_is_stable() {
        let k1 = cache_key("how does rust ownership work");
        let k2 = cache_key("how does rust ownership work");
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_store_and_check() {
        let mut cache = SearchCache { entries: HashMap::new(), path: PathBuf::from("/tmp/hydra_test_cache.json") };
        let hits = vec![crate::types::SearchHit {
            title: "Test".into(), url: "https://example.com".into(),
            snippet: "test".into(), content: None, score: 0.8,
            source: crate::types::EngineLabel::DuckDuckGo, confidence: 0.65,
        }];
        cache.store("rust ownership", &hits, None, ContentFocus::General);
        let result = cache.check("rust ownership", ContentFocus::General);
        assert!(result.is_some());
        assert!(result.unwrap().from_cache);
        let _ = std::fs::remove_file("/tmp/hydra_test_cache.json");
    }
}
