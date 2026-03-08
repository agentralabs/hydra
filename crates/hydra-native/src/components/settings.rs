//! Settings panel component data.

use serde::{Deserialize, Serialize};

use crate::state::hydra::{AppConfig, Theme};

/// A single settings field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsField {
    pub key: String,
    pub label: String,
    pub value: SettingsValue,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingsValue {
    Text(String),
    Bool(bool),
    Choice {
        selected: String,
        options: Vec<String>,
    },
}

/// Build settings fields from config
pub fn config_to_fields(config: &AppConfig) -> Vec<SettingsField> {
    vec![
        SettingsField {
            key: "server_url".into(),
            label: "Server URL".into(),
            value: SettingsValue::Text(config.server_url.clone()),
            description: "Hydra server endpoint".into(),
        },
        SettingsField {
            key: "theme".into(),
            label: "Theme".into(),
            value: SettingsValue::Choice {
                selected: match config.theme {
                    Theme::Dark => "dark".into(),
                    Theme::Light => "light".into(),
                },
                options: vec!["dark".into(), "light".into()],
            },
            description: "UI color scheme".into(),
        },
        SettingsField {
            key: "voice_enabled".into(),
            label: "Voice Input".into(),
            value: SettingsValue::Bool(config.voice_enabled),
            description: "Enable voice commands".into(),
        },
    ]
}

/// Apply a field update to config
pub fn apply_field(config: &mut AppConfig, key: &str, value: &SettingsValue) -> bool {
    match (key, value) {
        ("server_url", SettingsValue::Text(url)) => {
            config.server_url = url.clone();
            true
        }
        ("theme", SettingsValue::Choice { selected, .. }) => {
            config.theme = match selected.as_str() {
                "light" => Theme::Light,
                _ => Theme::Dark,
            };
            true
        }
        ("voice_enabled", SettingsValue::Bool(v)) => {
            config.voice_enabled = *v;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_to_fields() {
        let config = AppConfig::default();
        let fields = config_to_fields(&config);
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].key, "server_url");
    }

    #[test]
    fn test_apply_theme() {
        let mut config = AppConfig::default();
        assert_eq!(config.theme, Theme::Dark);
        apply_field(
            &mut config,
            "theme",
            &SettingsValue::Choice {
                selected: "light".into(),
                options: vec![],
            },
        );
        assert_eq!(config.theme, Theme::Light);
    }

    #[test]
    fn test_apply_voice() {
        let mut config = AppConfig::default();
        assert!(!config.voice_enabled);
        apply_field(&mut config, "voice_enabled", &SettingsValue::Bool(true));
        assert!(config.voice_enabled);
    }

    #[test]
    fn test_apply_unknown_key() {
        let mut config = AppConfig::default();
        let result = apply_field(&mut config, "unknown", &SettingsValue::Bool(true));
        assert!(!result);
    }
}
