//! Owner guardrail configuration — loaded from ~/.hydra/guardrails/boundaries.toml.
//! Default: fully permissive. Owner adds restrictions, never removes capabilities.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Owner-defined boundaries for Hydra's self-governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailConfig {
    /// Days without owner interaction before pausing proactive + evolution.
    /// None = disabled (default).
    pub dead_man_switch_days: Option<u64>,
    /// Glob patterns where evolution is allowed to write (empty = allow all).
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Paths that evolution can NEVER write to (always enforced).
    #[serde(default = "default_forbidden")]
    pub forbidden_paths: Vec<String>,
    /// Maximum lines per evolution-generated file.
    #[serde(default = "default_max_lines")]
    pub max_lines_per_evolution: usize,
    /// Blast radius level that requires owner approval for evolution.
    /// "Contained" | "Visible" | "Irreversible" | "Catastrophic"
    #[serde(default = "default_approval_level")]
    pub require_approval_above: String,
    /// Whether remote kill via HTTP API is enabled.
    #[serde(default = "default_true")]
    pub remote_kill_enabled: bool,
    /// PIN for HTTP kill endpoint (None = endpoint disabled).
    pub http_kill_pin: Option<String>,
}

fn default_forbidden() -> Vec<String> {
    vec![
        "guardrail/".into(),
        "security/".into(),
        "vault_crypto.rs".into(),
    ]
}
fn default_max_lines() -> usize { 400 }
fn default_approval_level() -> String { "Visible".into() }
fn default_true() -> bool { true }

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            dead_man_switch_days: None,
            allowed_paths: Vec::new(),
            forbidden_paths: default_forbidden(),
            max_lines_per_evolution: 400,
            require_approval_above: "Visible".into(),
            remote_kill_enabled: true,
            http_kill_pin: None,
        }
    }
}

impl GuardrailConfig {
    /// Load config from ~/.hydra/guardrails/boundaries.toml.
    /// Falls back to Default (fully permissive) on any error.
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => {
                    eprintln!("hydra-guardrail: config loaded from {}", path.display());
                    config
                }
                Err(e) => {
                    eprintln!("hydra-guardrail: config parse error (using defaults): {e}");
                    Self::default()
                }
            },
            Err(_) => Self::default(), // No config file = permissive
        }
    }

    /// Check if a path is allowed for evolution writes.
    /// Forbidden paths ALWAYS block. If allowed_paths is empty, all non-forbidden are allowed.
    pub fn is_path_allowed(&self, path: &str) -> bool {
        // Forbidden always wins
        if self.is_forbidden(path) { return false; }
        // If no allowed_paths specified, everything non-forbidden is OK
        if self.allowed_paths.is_empty() { return true; }
        // Check against allowed patterns
        self.allowed_paths.iter().any(|p| path.contains(p))
    }

    /// Check if a path targets the guardrail module itself (immutable core).
    pub fn is_self_modification(&self, path: &str) -> bool {
        let lower = path.to_lowercase();
        lower.contains("guardrail") || lower.contains("guard_rail")
    }

    fn is_forbidden(&self, path: &str) -> bool {
        self.forbidden_paths.iter().any(|f| path.contains(f))
            || self.is_self_modification(path)
    }

    /// Blast radius level as numeric for comparison.
    pub fn approval_threshold(&self) -> u8 {
        match self.require_approval_above.to_lowercase().as_str() {
            "contained" => 0,
            "visible" => 1,
            "irreversible" => 2,
            "catastrophic" => 3,
            _ => 1, // default to Visible
        }
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".hydra/guardrails/boundaries.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_permissive() {
        let config = GuardrailConfig::default();
        assert!(config.allowed_paths.is_empty());
        assert!(config.dead_man_switch_days.is_none());
        assert!(config.is_path_allowed("skills/auto_test/"));
    }

    #[test]
    fn forbidden_paths_always_block() {
        let config = GuardrailConfig::default();
        assert!(!config.is_path_allowed("guardrail/mod.rs"));
        assert!(!config.is_path_allowed("security/features.rs"));
        assert!(!config.is_path_allowed("vault_crypto.rs"));
    }

    #[test]
    fn self_modification_blocked() {
        let config = GuardrailConfig::default();
        assert!(config.is_self_modification("src/guardrail/mod.rs"));
        assert!(!config.is_self_modification("src/worker.rs"));
    }
}
