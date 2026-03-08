//! Profile data persisted to ~/.hydra/profile.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedProfile {
    pub user_name: Option<String>,
    pub voice_enabled: bool,
    pub onboarding_complete: bool,
    pub selected_model: Option<String>,
    pub api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub google_api_key: Option<String>,
    pub theme: Option<String>,
    pub auto_approve: bool,
    pub default_mode: Option<String>,
    pub sounds_enabled: bool,
    pub sound_volume: u8,
}

impl Default for PersistedProfile {
    fn default() -> Self {
        Self {
            user_name: None,
            voice_enabled: false,
            onboarding_complete: false,
            selected_model: None,
            api_key: None,
            anthropic_api_key: None,
            openai_api_key: None,
            google_api_key: None,
            theme: None,
            auto_approve: false,
            default_mode: None,
            sounds_enabled: true,
            sound_volume: 70,
        }
    }
}

fn profile_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".hydra").join("profile.json"))
}

pub fn load_profile() -> Option<PersistedProfile> {
    let path = profile_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_profile(profile: &PersistedProfile) {
    if let Some(path) = profile_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(profile) {
            let _ = std::fs::write(path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let p = PersistedProfile::default();
        assert!(p.user_name.is_none());
        assert!(!p.voice_enabled);
        assert!(!p.onboarding_complete);
        assert!(p.sounds_enabled);
        assert_eq!(p.sound_volume, 70);
    }

    #[test]
    fn test_profile_serialization_roundtrip() {
        let p = PersistedProfile {
            user_name: Some("Test".into()),
            voice_enabled: true,
            onboarding_complete: true,
            selected_model: Some("claude-sonnet-4-6".into()),
            api_key: None,
            anthropic_api_key: Some("sk-ant-test".into()),
            openai_api_key: None,
            google_api_key: None,
            theme: Some("dark".into()),
            auto_approve: false,
            default_mode: Some("companion".into()),
            sounds_enabled: true,
            sound_volume: 80,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PersistedProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_name.as_deref(), Some("Test"));
        assert!(back.voice_enabled);
        assert_eq!(back.sound_volume, 80);
    }
}
