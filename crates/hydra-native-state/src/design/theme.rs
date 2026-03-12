//! Theme definitions combining colors and typography into a cohesive look.

use serde::{Deserialize, Serialize};

use super::colors::DesignColors;

/// A complete visual theme for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTheme {
    pub name: String,
    pub bg: String,
    pub text: String,
    pub accent: String,
    pub is_dark: bool,
}

impl DesignTheme {
    /// Default dark theme — Hydra's primary look.
    pub fn dark() -> Self {
        Self {
            name: "dark".into(),
            bg: DesignColors::BG_PRIMARY.into(),
            text: DesignColors::WARM_WHITE.into(),
            accent: DesignColors::TRUST_BLUE.into(),
            is_dark: true,
        }
    }

    /// Light theme for daytime / preference.
    pub fn light() -> Self {
        Self {
            name: "light".into(),
            bg: DesignColors::WARM_WHITE.into(),
            text: DesignColors::SOFT_BLACK.into(),
            accent: DesignColors::TRUST_BLUE.into(),
            is_dark: false,
        }
    }

    /// High-contrast theme for accessibility.
    pub fn high_contrast() -> Self {
        Self {
            name: "high_contrast".into(),
            bg: "#000000".into(),
            text: "#FFFFFF".into(),
            accent: "#FFFF00".into(),
            is_dark: true,
        }
    }
}

impl Default for DesignTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::colors::is_valid_color;

    #[test]
    fn test_dark_theme() {
        let t = DesignTheme::dark();
        assert_eq!(t.name, "dark");
        assert!(t.is_dark);
        assert!(is_valid_color(&t.bg));
        assert!(is_valid_color(&t.text));
        assert!(is_valid_color(&t.accent));
    }

    #[test]
    fn test_light_theme() {
        let t = DesignTheme::light();
        assert_eq!(t.name, "light");
        assert!(!t.is_dark);
        assert!(is_valid_color(&t.bg));
        assert!(is_valid_color(&t.text));
        assert!(is_valid_color(&t.accent));
    }

    #[test]
    fn test_high_contrast_theme() {
        let t = DesignTheme::high_contrast();
        assert_eq!(t.name, "high_contrast");
        assert!(t.is_dark);
        assert!(is_valid_color(&t.bg));
        assert!(is_valid_color(&t.text));
        assert!(is_valid_color(&t.accent));
    }

    #[test]
    fn test_default_is_dark() {
        let t = DesignTheme::default();
        assert_eq!(t.name, "dark");
    }

    #[test]
    fn test_theme_serialization_roundtrip() {
        let t = DesignTheme::dark();
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("dark"));
        let back: DesignTheme = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "dark");
        assert!(back.is_dark);
    }
}
