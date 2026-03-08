use serde::{Deserialize, Serialize};

/// Interface mode controlling how Hydra presents itself
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceMode {
    /// Fully background — no visible UI
    Invisible,
    /// Floating companion widget
    Companion,
    /// Side-panel workspace integration
    Workspace,
    /// Full-screen immersive environment
    Immersive,
}

impl Default for InterfaceMode {
    fn default() -> Self {
        Self::Companion
    }
}

/// Color theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::System
    }
}

/// User preferences for Hydra behavior and appearance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Enable voice interaction
    pub voice_enabled: bool,
    /// Enable UI sounds
    pub sounds_enabled: bool,
    /// Sound volume (0.0 to 1.0)
    pub sound_volume: f32,
    /// Default interface mode
    pub default_mode: InterfaceMode,
    /// Color theme
    pub theme: Theme,
    /// Enable wake-word detection
    pub wake_word_enabled: bool,
    /// Automatically approve low-risk actions without prompting
    pub auto_approve_low_risk: bool,
    /// Preferred language (BCP-47 tag, e.g. "en", "es", "ja")
    pub language: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            voice_enabled: false,
            sounds_enabled: true,
            sound_volume: 0.7,
            default_mode: InterfaceMode::default(),
            theme: Theme::default(),
            wake_word_enabled: false,
            auto_approve_low_risk: false,
            language: "en".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = UserPreferences::default();
        assert!(!prefs.voice_enabled);
        assert!(prefs.sounds_enabled);
        assert_eq!(prefs.sound_volume, 0.7);
        assert_eq!(prefs.default_mode, InterfaceMode::Companion);
        assert_eq!(prefs.theme, Theme::System);
        assert!(!prefs.wake_word_enabled);
        assert!(!prefs.auto_approve_low_risk);
        assert_eq!(prefs.language, "en");
    }

    #[test]
    fn test_interface_mode_serde() {
        for mode in [InterfaceMode::Invisible, InterfaceMode::Companion, InterfaceMode::Workspace, InterfaceMode::Immersive] {
            let json = serde_json::to_string(&mode).unwrap();
            let restored: InterfaceMode = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, mode);
        }
    }

    #[test]
    fn test_theme_serde() {
        for theme in [Theme::Light, Theme::Dark, Theme::System] {
            let json = serde_json::to_string(&theme).unwrap();
            let restored: Theme = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, theme);
        }
    }

    #[test]
    fn test_preferences_serde_roundtrip() {
        let mut prefs = UserPreferences::default();
        prefs.voice_enabled = true;
        prefs.language = "ja".into();
        let json = serde_json::to_string(&prefs).unwrap();
        let restored: UserPreferences = serde_json::from_str(&json).unwrap();
        assert!(restored.voice_enabled);
        assert_eq!(restored.language, "ja");
    }

    #[test]
    fn test_default_interface_mode() {
        assert_eq!(InterfaceMode::default(), InterfaceMode::Companion);
    }

    #[test]
    fn test_default_theme() {
        assert_eq!(Theme::default(), Theme::System);
    }
}
