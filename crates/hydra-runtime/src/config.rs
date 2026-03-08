use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Resource profile for Hydra
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceProfile {
    Minimal,
    Standard,
    Performance,
    Unlimited,
}

/// LLM provider configuration within the TOML config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmConfigSection {
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub default_provider: Option<String>,
    pub perception_model: Option<String>,
    pub thinking_model: Option<String>,
    pub decision_model: Option<String>,
}

/// Limits and budgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub token_budget: u64,
    pub max_concurrent_runs: usize,
    pub approval_timeout_secs: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            token_budget: 100_000,
            max_concurrent_runs: 10,
            approval_timeout_secs: 300,
        }
    }
}

/// Full runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraRuntimeConfig {
    pub data_dir: PathBuf,
    pub profile: ResourceProfile,
    pub voice_enabled: bool,
    pub wake_word: String,
    pub api_port: u16,
    pub log_level: String,
    pub server_mode: bool,
    #[serde(default)]
    pub llm: LlmConfigSection,
    #[serde(default)]
    pub limits: LimitsConfig,
}

impl Default for HydraRuntimeConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            profile: ResourceProfile::Standard,
            voice_enabled: false,
            wake_word: "hey hydra".into(),
            api_port: 7777,
            log_level: "info".into(),
            server_mode: false,
            llm: LlmConfigSection::default(),
            limits: LimitsConfig::default(),
        }
    }
}

impl HydraRuntimeConfig {
    /// Load config from file, falling back to defaults
    pub fn load(path: Option<&PathBuf>) -> Self {
        if let Some(path) = path {
            if let Ok(contents) = std::fs::read_to_string(path) {
                if let Ok(config) = toml::from_str::<Self>(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Load from the default config path (~/.hydra/config.toml)
    pub fn load_default() -> Self {
        let config_path = default_data_dir().join("config.toml");
        let mut config = Self::load(Some(&config_path));
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides (highest priority)
    pub fn apply_env_overrides(&mut self) {
        if let Ok(dir) = std::env::var("HYDRA_DATA_DIR") {
            self.data_dir = PathBuf::from(dir);
        }
        if let Ok(profile) = std::env::var("HYDRA_PROFILE") {
            self.profile = match profile.to_lowercase().as_str() {
                "minimal" => ResourceProfile::Minimal,
                "standard" => ResourceProfile::Standard,
                "performance" => ResourceProfile::Performance,
                "unlimited" => ResourceProfile::Unlimited,
                _ => self.profile,
            };
        }
        if let Ok(voice) = std::env::var("HYDRA_VOICE") {
            self.voice_enabled = voice == "true" || voice == "1";
        }
        if let Ok(port) = std::env::var("HYDRA_PORT") {
            if let Ok(p) = port.parse() {
                self.api_port = p;
            }
        }
        if let Ok(level) = std::env::var("HYDRA_LOG_LEVEL") {
            self.log_level = level;
        }

        // LLM keys: env vars override config.toml
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                self.llm.anthropic_api_key = Some(key);
            }
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                self.llm.openai_api_key = Some(key);
            }
        }

        // Limits overrides
        if let Ok(budget) = std::env::var("HYDRA_TOKEN_BUDGET") {
            if let Ok(b) = budget.parse() {
                self.limits.token_budget = b;
            }
        }
        if let Ok(max) = std::env::var("HYDRA_MAX_CONCURRENT_RUNS") {
            if let Ok(m) = max.parse() {
                self.limits.max_concurrent_runs = m;
            }
        }
    }

    /// Build an LlmConfig from this runtime config
    pub fn to_llm_config(&self) -> hydra_model::LlmConfig {
        hydra_model::LlmConfig {
            anthropic_api_key: self.llm.anthropic_api_key.clone(),
            openai_api_key: self.llm.openai_api_key.clone(),
            anthropic_base_url: "https://api.anthropic.com".into(),
            openai_base_url: "https://api.openai.com".into(),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.api_port == 0 {
            errors.push("API port cannot be 0. Use a port between 1024 and 65535.".into());
        }
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.as_str()) {
            errors.push(format!(
                "Invalid log level '{}'. Valid levels: {}.",
                self.log_level,
                valid_levels.join(", ")
            ));
        }
        if self.limits.token_budget == 0 {
            errors.push("Token budget cannot be 0.".into());
        }
        if self.limits.max_concurrent_runs == 0 {
            errors.push("Max concurrent runs cannot be 0.".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get the checkpoint file path
    pub fn checkpoint_path(&self) -> PathBuf {
        self.data_dir.join("checkpoint.json")
    }
}

fn default_data_dir() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".hydra"))
        .unwrap_or_else(|_| PathBuf::from("/tmp/.hydra"))
}
