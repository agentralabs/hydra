//! Persisted settings state with natural language mutation (Step 4.9).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Compression level for context windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionLevel {
    Aggressive,
    Balanced,
    Minimal,
}

/// Strategy for routing tasks across sisters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    Parallel,
    Sequential,
}

/// Persisted settings that the user can mutate via natural language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsStore {
    pub model_provider: String,
    pub model_name: String,
    pub api_keys: HashMap<String, String>,
    pub creativity: f32,
    pub max_tokens_per_op: u32,
    pub intent_cache_enabled: bool,
    pub intent_cache_ttl_secs: u64,
    pub belief_revision_enabled: bool,
    pub context_compression: CompressionLevel,
    pub sister_routing: RoutingStrategy,
    pub sister_timeout_secs: u64,
    pub proactive_enabled: bool,
    pub dream_state_enabled: bool,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self {
            model_provider: "anthropic".into(),
            model_name: "claude-sonnet-4-20250514".into(),
            api_keys: HashMap::new(),
            creativity: 0.5,
            max_tokens_per_op: 4096,
            intent_cache_enabled: true,
            intent_cache_ttl_secs: 300,
            belief_revision_enabled: true,
            context_compression: CompressionLevel::Balanced,
            sister_routing: RoutingStrategy::Parallel,
            sister_timeout_secs: 30,
            proactive_enabled: false,
            dream_state_enabled: false,
        }
    }
}

impl SettingsStore {
    /// LOCAL-ONLY settings parser. Runs without LLM as a convenience shortcut.
    /// For full natural language understanding, the cognitive loop handles settings via LLM.
    /// These patterns are intentionally hardcoded as pre-LLM shortcuts.
    ///
    /// Returns a confirmation message on success, or `None` if the instruction was not understood.
    pub fn apply_natural_language(&mut self, instruction: &str) -> Option<String> {
        let lower = instruction.to_lowercase();

        // Creativity adjustments
        if lower.contains("more creative") || lower.contains("increase creativity") {
            self.creativity = (self.creativity + 0.15).min(1.0);
            return Some(format!("Creativity increased to {:.2}", self.creativity));
        }
        if lower.contains("less creative") || lower.contains("decrease creativity") {
            self.creativity = (self.creativity - 0.15).max(0.0);
            return Some(format!("Creativity decreased to {:.2}", self.creativity));
        }

        // Provider switching
        if lower.contains("use openai") || lower.contains("switch to openai") {
            self.model_provider = "openai".into();
            self.model_name = "gpt-4o".into();
            return Some("Switched to OpenAI (gpt-4o)".into());
        }
        if lower.contains("use anthropic") || lower.contains("switch to anthropic") {
            self.model_provider = "anthropic".into();
            self.model_name = "claude-sonnet-4-20250514".into();
            return Some("Switched to Anthropic (claude-sonnet-4-20250514)".into());
        }

        // API key storage
        if lower.contains("remember") && lower.contains("key") {
            // Extract key value — last whitespace-separated token
            if let Some(key_value) = instruction.split_whitespace().last() {
                let provider = self.model_provider.clone();
                self.api_keys.insert(provider.clone(), key_value.to_string());
                return Some(format!("API key stored for {}", provider));
            }
        }

        // Timeout
        if lower.contains("timeout") {
            if let Some(secs) = extract_number(&lower) {
                self.sister_timeout_secs = secs;
                return Some(format!("Sister timeout set to {} seconds", secs));
            }
        }

        // Max tokens
        if lower.contains("max tokens") || lower.contains("token limit") {
            if let Some(n) = extract_number(&lower) {
                self.max_tokens_per_op = n as u32;
                return Some(format!("Max tokens per operation set to {}", n));
            }
        }

        // Cache TTL
        if lower.contains("cache ttl") || lower.contains("cache lifetime") {
            if let Some(n) = extract_number(&lower) {
                self.intent_cache_ttl_secs = n;
                return Some(format!("Intent cache TTL set to {} seconds", n));
            }
        }

        // Boolean toggles — dream state
        if lower.contains("enable dream") || lower.contains("turn on dream") {
            self.dream_state_enabled = true;
            return Some("Dream state enabled".into());
        }
        if lower.contains("disable dream") || lower.contains("turn off dream") {
            self.dream_state_enabled = false;
            return Some("Dream state disabled".into());
        }

        // Boolean toggles — proactive
        if lower.contains("enable proactive") || lower.contains("turn on proactive") {
            self.proactive_enabled = true;
            return Some("Proactive mode enabled".into());
        }
        if lower.contains("disable proactive") || lower.contains("turn off proactive") {
            self.proactive_enabled = false;
            return Some("Proactive mode disabled".into());
        }

        // Boolean toggles — intent cache
        if lower.contains("enable cache") || lower.contains("turn on cache") {
            self.intent_cache_enabled = true;
            return Some("Intent cache enabled".into());
        }
        if lower.contains("disable cache") || lower.contains("turn off cache") {
            self.intent_cache_enabled = false;
            return Some("Intent cache disabled".into());
        }

        // Boolean toggles — belief revision
        if lower.contains("enable belief") || lower.contains("turn on belief") {
            self.belief_revision_enabled = true;
            return Some("Belief revision enabled".into());
        }
        if lower.contains("disable belief") || lower.contains("turn off belief") {
            self.belief_revision_enabled = false;
            return Some("Belief revision disabled".into());
        }

        // Compression level
        if lower.contains("aggressive compression") {
            self.context_compression = CompressionLevel::Aggressive;
            return Some("Context compression set to aggressive".into());
        }
        if lower.contains("minimal compression") {
            self.context_compression = CompressionLevel::Minimal;
            return Some("Context compression set to minimal".into());
        }
        if lower.contains("balanced compression") {
            self.context_compression = CompressionLevel::Balanced;
            return Some("Context compression set to balanced".into());
        }

        // Routing strategy
        if lower.contains("sequential routing") || lower.contains("route sequentially") {
            self.sister_routing = RoutingStrategy::Sequential;
            return Some("Sister routing set to sequential".into());
        }
        if lower.contains("parallel routing") || lower.contains("route in parallel") {
            self.sister_routing = RoutingStrategy::Parallel;
            return Some("Sister routing set to parallel".into());
        }

        None
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Deserialize from a JSON string.
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Persist settings to disk.
    pub fn save(&self, path: &Path) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    /// Load settings from disk.
    pub fn load(path: &Path) -> Option<Self> {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }
}

/// Extract the first integer-like number from a string.
fn extract_number(s: &str) -> Option<u64> {
    s.split_whitespace()
        .filter_map(|token| token.trim_matches(|c: char| !c.is_ascii_digit()).parse::<u64>().ok())
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let s = SettingsStore::default();
        assert_eq!(s.model_provider, "anthropic");
        assert!((s.creativity - 0.5).abs() < f32::EPSILON);
        assert_eq!(s.max_tokens_per_op, 4096);
        assert!(s.intent_cache_enabled);
        assert!(!s.dream_state_enabled);
        assert!(!s.proactive_enabled);
        assert_eq!(s.context_compression, CompressionLevel::Balanced);
        assert_eq!(s.sister_routing, RoutingStrategy::Parallel);
    }

    #[test]
    fn test_creativity_increase() {
        let mut s = SettingsStore::default();
        let msg = s.apply_natural_language("be more creative").unwrap();
        assert!(s.creativity > 0.5);
        assert!(msg.contains("increased"));
    }

    #[test]
    fn test_creativity_decrease() {
        let mut s = SettingsStore::default();
        let msg = s.apply_natural_language("be less creative").unwrap();
        assert!(s.creativity < 0.5);
        assert!(msg.contains("decreased"));
    }

    #[test]
    fn test_creativity_clamp() {
        let mut s = SettingsStore::default();
        s.creativity = 0.95;
        s.apply_natural_language("be more creative");
        assert!(s.creativity <= 1.0);

        s.creativity = 0.05;
        s.apply_natural_language("be less creative");
        assert!(s.creativity >= 0.0);
    }

    #[test]
    fn test_switch_provider() {
        let mut s = SettingsStore::default();
        let msg = s.apply_natural_language("use OpenAI").unwrap();
        assert_eq!(s.model_provider, "openai");
        assert_eq!(s.model_name, "gpt-4o");
        assert!(msg.contains("OpenAI"));

        let msg = s.apply_natural_language("switch to anthropic").unwrap();
        assert_eq!(s.model_provider, "anthropic");
        assert!(msg.contains("Anthropic"));
    }

    #[test]
    fn test_remember_key() {
        let mut s = SettingsStore::default();
        let msg = s.apply_natural_language("remember my key ABC123").unwrap();
        assert_eq!(s.api_keys.get("anthropic"), Some(&"ABC123".to_string()));
        assert!(msg.contains("stored"));
    }

    #[test]
    fn test_set_timeout() {
        let mut s = SettingsStore::default();
        let msg = s.apply_natural_language("set timeout to 60 seconds").unwrap();
        assert_eq!(s.sister_timeout_secs, 60);
        assert!(msg.contains("60"));
    }

    #[test]
    fn test_toggle_dream_state() {
        let mut s = SettingsStore::default();
        assert!(!s.dream_state_enabled);
        let msg = s.apply_natural_language("enable dream state").unwrap();
        assert!(s.dream_state_enabled);
        assert!(msg.contains("enabled"));

        let msg = s.apply_natural_language("disable dream state").unwrap();
        assert!(!s.dream_state_enabled);
        assert!(msg.contains("disabled"));
    }

    #[test]
    fn test_unrecognized_instruction() {
        let mut s = SettingsStore::default();
        let result = s.apply_natural_language("tell me a joke");
        assert!(result.is_none());
    }

    #[test]
    fn test_json_roundtrip() {
        let mut s = SettingsStore::default();
        s.creativity = 0.8;
        s.api_keys.insert("test".into(), "key123".into());
        let json = s.to_json();
        let loaded = SettingsStore::from_json(&json).unwrap();
        assert!((loaded.creativity - 0.8).abs() < f32::EPSILON);
        assert_eq!(loaded.api_keys.get("test"), Some(&"key123".to_string()));
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("hydra_test_settings");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");

        let mut s = SettingsStore::default();
        s.dream_state_enabled = true;
        s.sister_timeout_secs = 99;
        s.save(&path);

        let loaded = SettingsStore::load(&path).unwrap();
        assert!(loaded.dream_state_enabled);
        assert_eq!(loaded.sister_timeout_secs, 99);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file() {
        let result = SettingsStore::load(Path::new("/tmp/nonexistent_hydra_settings.json"));
        assert!(result.is_none());
    }

    #[test]
    fn test_compression_and_routing() {
        let mut s = SettingsStore::default();
        s.apply_natural_language("aggressive compression");
        assert_eq!(s.context_compression, CompressionLevel::Aggressive);

        s.apply_natural_language("sequential routing");
        assert_eq!(s.sister_routing, RoutingStrategy::Sequential);
    }

    #[test]
    fn test_serialization_enums() {
        let levels = [CompressionLevel::Aggressive, CompressionLevel::Balanced, CompressionLevel::Minimal];
        for l in &levels {
            let json = serde_json::to_string(l).unwrap();
            let back: CompressionLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(*l, back);
        }

        let strategies = [RoutingStrategy::Parallel, RoutingStrategy::Sequential];
        for s in &strategies {
            let json = serde_json::to_string(s).unwrap();
            let back: RoutingStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
