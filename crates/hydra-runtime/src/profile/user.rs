use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::preferences::UserPreferences;
use super::storage::{ProfileStorage, ProfileStorageError};

/// A user's profile, persisted to ~/.hydra/profile.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Display name (optional — user may skip during onboarding)
    pub name: Option<String>,
    /// When the profile was first created
    pub created_at: DateTime<Utc>,
    /// Whether the user has completed the onboarding flow
    pub onboarding_complete: bool,
    /// User preferences
    pub preferences: UserPreferences,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: None,
            created_at: Utc::now(),
            onboarding_complete: false,
            preferences: UserPreferences::default(),
        }
    }
}

impl UserProfile {
    /// Load profile from the default path (~/.hydra/profile.json).
    /// Returns a fresh default profile if the file does not exist.
    pub fn load() -> Result<Self, ProfileStorageError> {
        ProfileStorage::load_default()
    }

    /// Load profile from a custom path.
    pub fn load_from(path: &std::path::Path) -> Result<Self, ProfileStorageError> {
        ProfileStorage::load(path)
    }

    /// Save profile to the default path (~/.hydra/profile.json).
    pub fn save(&self) -> Result<(), ProfileStorageError> {
        ProfileStorage::save_default(self)
    }

    /// Save profile to a custom path.
    pub fn save_to(&self, path: &std::path::Path) -> Result<(), ProfileStorageError> {
        ProfileStorage::save(path, self)
    }

    /// True when the user has not yet completed onboarding
    pub fn is_first_run(&self) -> bool {
        !self.onboarding_complete
    }

    /// Set the user's display name
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }

    /// Produce a greeting string
    pub fn get_greeting(&self) -> String {
        match &self.name {
            Some(name) if !name.is_empty() => format!("Hi {}!", name),
            _ => "Hi there!".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let profile = UserProfile::default();
        assert!(profile.name.is_none());
        assert!(!profile.onboarding_complete);
        assert!(profile.is_first_run());
    }

    #[test]
    fn test_set_name() {
        let mut profile = UserProfile::default();
        profile.set_name("Alice");
        assert_eq!(profile.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_greeting_with_name() {
        let mut profile = UserProfile::default();
        profile.set_name("Bob");
        assert_eq!(profile.get_greeting(), "Hi Bob!");
    }

    #[test]
    fn test_greeting_without_name() {
        let profile = UserProfile::default();
        assert_eq!(profile.get_greeting(), "Hi there!");
    }

    #[test]
    fn test_greeting_empty_name() {
        let mut profile = UserProfile::default();
        profile.name = Some("".into());
        assert_eq!(profile.get_greeting(), "Hi there!");
    }

    #[test]
    fn test_is_first_run_true() {
        let profile = UserProfile::default();
        assert!(profile.is_first_run());
    }

    #[test]
    fn test_is_first_run_false() {
        let mut profile = UserProfile::default();
        profile.onboarding_complete = true;
        assert!(!profile.is_first_run());
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut profile = UserProfile::default();
        profile.set_name("Test");
        profile.onboarding_complete = true;
        let json = serde_json::to_string(&profile).unwrap();
        let restored: UserProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, Some("Test".into()));
        assert!(restored.onboarding_complete);
    }

    #[test]
    fn test_save_and_load_from_path() {
        let dir = std::env::temp_dir().join(format!("hydra_profile_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_profile.json");

        let mut profile = UserProfile::default();
        profile.set_name("TestUser");
        profile.save_to(&path).unwrap();

        let loaded = UserProfile::load_from(&path).unwrap();
        assert_eq!(loaded.name, Some("TestUser".into()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let path = std::path::Path::new("/tmp/hydra_nonexistent_profile_12345.json");
        let profile = UserProfile::load_from(path).unwrap();
        assert!(profile.name.is_none());
        assert!(!profile.onboarding_complete);
    }
}
