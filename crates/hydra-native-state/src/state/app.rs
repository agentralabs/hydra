//! Global app state with mode switching.

use serde::{Deserialize, Serialize};

use crate::design::theme::DesignTheme;

/// The four application display modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Invisible,
    Companion,
    Workspace,
    Immersive,
}

/// Top-level application state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub mode: AppMode,
    pub user_name: Option<String>,
    pub onboarding_complete: bool,
    pub voice_enabled: bool,
    pub theme: DesignTheme,
    pub sound_enabled: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Companion,
            user_name: None,
            onboarding_complete: false,
            voice_enabled: false,
            theme: DesignTheme::dark(),
            sound_enabled: true,
        }
    }

    /// Switch to a different display mode.
    pub fn switch_mode(&mut self, mode: AppMode) {
        self.mode = mode;
    }

    /// Personalized greeting for the user.
    pub fn greeting(&self) -> String {
        match &self.user_name {
            Some(name) => format!("Hi {}!", name),
            None => "Hi there!".to_string(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let s = AppState::new();
        assert_eq!(s.mode, AppMode::Companion);
        assert!(s.user_name.is_none());
        assert!(!s.onboarding_complete);
        assert!(!s.voice_enabled);
        assert!(s.sound_enabled);
        assert_eq!(s.theme.name, "dark");
    }

    #[test]
    fn test_switch_mode() {
        let mut s = AppState::new();
        s.switch_mode(AppMode::Workspace);
        assert_eq!(s.mode, AppMode::Workspace);
        s.switch_mode(AppMode::Immersive);
        assert_eq!(s.mode, AppMode::Immersive);
        s.switch_mode(AppMode::Invisible);
        assert_eq!(s.mode, AppMode::Invisible);
    }

    #[test]
    fn test_greeting_with_name() {
        let mut s = AppState::new();
        s.user_name = Some("Sarah".to_string());
        assert_eq!(s.greeting(), "Hi Sarah!");
    }

    #[test]
    fn test_greeting_without_name() {
        let s = AppState::new();
        assert_eq!(s.greeting(), "Hi there!");
    }

    #[test]
    fn test_serialization() {
        let s = AppState::new();
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("companion"));
        let back: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mode, AppMode::Companion);
    }
}
