//! Constitution cache — persists page constitutions for instant repeat visits.
//! Same pattern as hydra-web/cache.rs. TTL-aware, semantic URL matching.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::PageConstitution;

const CACHE_TTL_SECS: u64 = 3600; // 1 hour for dynamic pages

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    constitution: PageConstitution,
    stored_at: chrono::DateTime<chrono::Utc>,
}

/// Persistent constitution cache.
pub struct ConstitutionCache {
    entries: HashMap<String, CacheEntry>,
    path: PathBuf,
}

impl ConstitutionCache {
    pub fn new() -> Self {
        let path = dirs::home_dir()
            .unwrap_or_default()
            .join(".hydra/data/nav_constitutions.json");
        let entries = load(&path);
        Self { entries, path }
    }

    /// Check cache for a matching URL.
    pub fn check(&self, url: &str) -> Option<&PageConstitution> {
        let key = url_key(url);
        let entry = self.entries.get(&key)?;
        let age = (chrono::Utc::now() - entry.stored_at).num_seconds() as u64;
        if age > CACHE_TTL_SECS { return None; }
        Some(&entry.constitution)
    }

    /// Store a constitution.
    pub fn store(&mut self, constitution: &PageConstitution) {
        let key = url_key(&constitution.url);
        self.entries.insert(key, CacheEntry {
            constitution: constitution.clone(),
            stored_at: chrono::Utc::now(),
        });
        self.persist();
    }

    /// Evict stale entries.
    pub fn evict_stale(&mut self) {
        let now = chrono::Utc::now();
        self.entries.retain(|_, e| {
            (now - e.stored_at).num_seconds() as u64 <= CACHE_TTL_SECS * 2
        });
        self.persist();
    }

    fn persist(&self) {
        if let Some(parent) = self.path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Ok(json) = serde_json::to_string(&self.entries) {
            if let Err(e) = std::fs::write(&self.path, json) {
                eprintln!("hydra-semantic-nav: cache persist failed: {e}");
            }
        }
    }
}

impl Default for ConstitutionCache {
    fn default() -> Self { Self::new() }
}

/// Normalize URL to a cache key (domain + path pattern).
fn url_key(url: &str) -> String {
    let url = url.split('?').next().unwrap_or(url); // strip query params
    let url = url.split('#').next().unwrap_or(url); // strip fragment
    url.to_lowercase()
        .replace("https://", "")
        .replace("http://", "")
        .replace("www.", "")
        .trim_end_matches('/')
        .to_string()
}

fn load(path: &PathBuf) -> HashMap<String, CacheEntry> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_key_normalizes() {
        assert_eq!(url_key("https://www.Example.com/page?q=1#top"), "example.com/page");
        assert_eq!(url_key("http://github.com/user/repo/"), "github.com/user/repo");
    }

    #[test]
    fn store_and_check() {
        let mut cache = ConstitutionCache {
            entries: HashMap::new(),
            path: PathBuf::from("/tmp/hydra_nav_cache_test.json"),
        };
        let constitution = PageConstitution {
            url: "https://example.com/test".into(), title: "Test".into(),
            elements: vec![], forms: vec![], navigation: vec![],
            primary_action: None, search_input: None, guards: vec![],
            parsed_at: chrono::Utc::now(),
        };
        cache.store(&constitution);
        assert!(cache.check("https://example.com/test").is_some());
        assert!(cache.check("https://other.com/test").is_none());
        let _ = std::fs::remove_file("/tmp/hydra_nav_cache_test.json");
    }
}
