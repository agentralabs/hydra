//! User preferences for personalization and accessibility.

use serde::{Deserialize, Serialize};

/// Persistent user preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub name: Option<String>,
    pub voice_enabled: bool,
    pub sound_enabled: bool,
    pub theme: String,
    /// Font size multiplier for accessibility (1.0 = default).
    pub font_size_multiplier: f32,
    pub high_contrast: bool,
}

impl UserPreferences {
    /// Friendly display name — falls back to "friend".
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("friend")
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            name: None,
            voice_enabled: false,
            sound_enabled: true,
            theme: "dark".to_string(),
            font_size_multiplier: 1.0,
            high_contrast: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let p = UserPreferences::default();
        assert!(p.name.is_none());
        assert!(!p.voice_enabled);
        assert!(p.sound_enabled);
        assert_eq!(p.theme, "dark");
        assert!((p.font_size_multiplier - 1.0).abs() < f32::EPSILON);
        assert!(!p.high_contrast);
    }

    #[test]
    fn test_display_name_with_name() {
        let p = UserPreferences {
            name: Some("Ada".to_string()),
            ..Default::default()
        };
        assert_eq!(p.display_name(), "Ada");
    }

    #[test]
    fn test_display_name_without_name() {
        let p = UserPreferences::default();
        assert_eq!(p.display_name(), "friend");
    }

    #[test]
    fn test_serialization() {
        let p = UserPreferences::default();
        let json = serde_json::to_string(&p).unwrap();
        let back: UserPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, "dark");
        assert_eq!(back.display_name(), "friend");
    }
}
