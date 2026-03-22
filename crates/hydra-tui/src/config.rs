//! Config file — ~/.hydra/config.toml persistence.
//!
//! Loads on startup, saves on /settings changes.
//! All settings have sensible defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// All user-configurable TUI settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HydraConfig {
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub voice: VoiceConfig,
    #[serde(default)]
    pub companion: CompanionConfig,
}

/// TUI appearance and behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Theme: "dark" or "light".
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Whether to render markdown in responses.
    #[serde(default = "default_true")]
    pub markdown: bool,
    /// Whether to stream responses token-by-token.
    #[serde(default = "default_true")]
    pub streaming: bool,
    /// Output pacer speed multiplier (1.0 = normal).
    #[serde(default = "default_speed")]
    pub pacer_speed: f64,
    /// Maximum input history entries.
    #[serde(default = "default_history")]
    pub max_history: usize,
}

/// Voice system settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Voice mode: "push-to-talk" or "always-on".
    #[serde(default = "default_voice_mode")]
    pub mode: String,
    /// Whether voice is enabled.
    #[serde(default)]
    pub enabled: bool,
}

/// Companion system settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionConfig {
    /// Whether companion is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Default autonomy level: "report", "confirm", "summarize", "auto".
    #[serde(default = "default_autonomy")]
    pub autonomy: String,
}

fn default_theme() -> String { "dark".into() }
fn default_true() -> bool { true }
fn default_speed() -> f64 { 1.0 }
fn default_history() -> usize { 100 }
fn default_voice_mode() -> String { "push-to-talk".into() }
fn default_autonomy() -> String { "confirm".into() }

// HydraConfig derives Default since all fields implement Default.

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            markdown: true,
            streaming: true,
            pacer_speed: 1.0,
            max_history: 100,
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            mode: default_voice_mode(),
            enabled: false,
        }
    }
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            autonomy: default_autonomy(),
        }
    }
}

impl HydraConfig {
    /// Load config from ~/.hydra/config.toml, or return defaults.
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("hydra: config parse error (using defaults): {e}");
                    }
                },
                Err(e) => {
                    eprintln!("hydra: config read error (using defaults): {e}");
                }
            }
        }
        Self::default()
    }

    /// Save config to ~/.hydra/config.toml.
    pub fn save(&self) -> Result<(), String> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir failed: {e}"))?;
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| format!("serialize failed: {e}"))?;
        std::fs::write(&path, toml_str)
            .map_err(|e| format!("write failed: {e}"))?;
        Ok(())
    }

    /// Apply a setting by key=value. Returns description of what changed.
    pub fn apply_setting(&mut self, key: &str, value: &str) -> Result<String, String> {
        match key {
            "theme" => {
                match value {
                    "dark" | "light" => {
                        self.tui.theme = value.to_string();
                        Ok(format!("theme set to: {value}"))
                    }
                    _ => Err("theme must be 'dark' or 'light'".into()),
                }
            }
            "markdown" => {
                self.tui.markdown = parse_bool(value)?;
                Ok(format!("markdown set to: {}", self.tui.markdown))
            }
            "streaming" => {
                self.tui.streaming = parse_bool(value)?;
                Ok(format!("streaming set to: {}", self.tui.streaming))
            }
            "pacer_speed" => {
                let speed: f64 = value.parse()
                    .map_err(|_| "pacer_speed must be a number (e.g., 1.0, 2.0)")?;
                if !(0.1..=10.0).contains(&speed) {
                    return Err("pacer_speed must be between 0.1 and 10.0".into());
                }
                self.tui.pacer_speed = speed;
                Ok(format!("pacer_speed set to: {speed}"))
            }
            "voice" => {
                self.voice.enabled = parse_bool(value)?;
                Ok(format!("voice set to: {}", self.voice.enabled))
            }
            "companion" => {
                self.companion.enabled = parse_bool(value)?;
                Ok(format!("companion set to: {}", self.companion.enabled))
            }
            _ => Err(format!("unknown setting: {key}")),
        }
    }

    /// Format current settings for display.
    pub fn display(&self) -> Vec<String> {
        vec![
            format!("  theme:       {}", self.tui.theme),
            format!("  markdown:    {}", self.tui.markdown),
            format!("  streaming:   {}", self.tui.streaming),
            format!("  pacer_speed: {}", self.tui.pacer_speed),
            format!("  max_history: {}", self.tui.max_history),
            format!("  voice:       {} (mode: {})", self.voice.enabled, self.voice.mode),
            format!("  companion:   {} (autonomy: {})", self.companion.enabled, self.companion.autonomy),
        ]
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err(format!("expected true/false, got: {value}")),
    }
}

/// Return the path to ~/.hydra/config.toml.
fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = HydraConfig::default();
        assert_eq!(config.tui.theme, "dark");
        assert!(config.tui.markdown);
        assert!(config.tui.streaming);
        assert_eq!(config.tui.pacer_speed, 1.0);
        assert!(!config.voice.enabled);
        assert!(!config.companion.enabled);
    }

    #[test]
    fn apply_theme_setting() {
        let mut config = HydraConfig::default();
        assert!(config.apply_setting("theme", "light").is_ok());
        assert_eq!(config.tui.theme, "light");
        assert!(config.apply_setting("theme", "invalid").is_err());
    }

    #[test]
    fn apply_speed_setting() {
        let mut config = HydraConfig::default();
        assert!(config.apply_setting("pacer_speed", "2.0").is_ok());
        assert_eq!(config.tui.pacer_speed, 2.0);
        assert!(config.apply_setting("pacer_speed", "0.05").is_err());
        assert!(config.apply_setting("pacer_speed", "11.0").is_err());
    }
}
