//! Profile data persisted to ~/.hydra/profile.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════
// PHASE 3, C2: Autonomy Slider — 5 user-facing levels
// ═══════════════════════════════════════════════════════════

/// User-facing autonomy level — controls how much Hydra acts independently.
/// Maps to the internal `hydra_autonomy::AutonomyLevel` but provides a
/// simpler 1–5 numeric scale for user configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserAutonomyLevel {
    /// Level 1: Ask before everything, even low-risk actions
    Supervised = 1,
    /// Level 2: Ask before medium+ risk, act on low risk alone
    Cautious = 2,
    /// Level 3: Ask before high+ risk (DEFAULT)
    Balanced = 3,
    /// Level 4: Only ask before critical/irreversible actions
    Autonomous = 4,
    /// Level 5: Act on everything, report after (requires explicit opt-in)
    FullAuto = 5,
}

impl UserAutonomyLevel {
    /// Whether a given risk level requires user approval at this autonomy setting.
    pub fn requires_approval_for_risk(&self, risk: &str) -> bool {
        match (self, risk) {
            (Self::Supervised, _) => true,
            (Self::Cautious, "medium" | "high" | "critical") => true,
            (Self::Balanced, "high" | "critical") => true,
            (Self::Autonomous, "critical") => true,
            (Self::FullAuto, _) => false,
            _ => false,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Supervised => "Supervised (ask everything)",
            Self::Cautious   => "Cautious (ask medium+)",
            Self::Balanced   => "Balanced (ask high+)",
            Self::Autonomous => "Autonomous (ask critical only)",
            Self::FullAuto   => "Full Auto (act and report)",
        }
    }

    /// Numeric level (1–5).
    pub fn level(&self) -> u8 {
        *self as u8
    }

    /// Parse from a numeric string "1"–"5".
    pub fn from_level(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Supervised),
            2 => Some(Self::Cautious),
            3 => Some(Self::Balanced),
            4 => Some(Self::Autonomous),
            5 => Some(Self::FullAuto),
            _ => None,
        }
    }
}

impl Default for UserAutonomyLevel {
    fn default() -> Self {
        Self::Balanced
    }
}

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
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Phase 3, C2: User-configured autonomy level (1–5).
    #[serde(default)]
    pub autonomy_level: UserAutonomyLevel,
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
            working_directory: None,
            autonomy_level: UserAutonomyLevel::default(),
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
        assert_eq!(p.autonomy_level, UserAutonomyLevel::Balanced);
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
            working_directory: Some("/tmp/test-project".into()),
            autonomy_level: UserAutonomyLevel::Autonomous,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PersistedProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_name.as_deref(), Some("Test"));
        assert!(back.voice_enabled);
        assert_eq!(back.sound_volume, 80);
        assert_eq!(back.autonomy_level, UserAutonomyLevel::Autonomous);
    }

    // ═══════════════════════════════════════════════════════════
    // PHASE 3, C2: AUTONOMY SLIDER TESTS
    // ═══════════════════════════════════════════════════════════

    #[test]
    fn test_autonomy_supervised_asks_everything() {
        let level = UserAutonomyLevel::Supervised;
        assert!(level.requires_approval_for_risk("none"));
        assert!(level.requires_approval_for_risk("low"));
        assert!(level.requires_approval_for_risk("medium"));
        assert!(level.requires_approval_for_risk("high"));
        assert!(level.requires_approval_for_risk("critical"));
    }

    #[test]
    fn test_autonomy_cautious_asks_medium_plus() {
        let level = UserAutonomyLevel::Cautious;
        assert!(!level.requires_approval_for_risk("none"));
        assert!(!level.requires_approval_for_risk("low"));
        assert!(level.requires_approval_for_risk("medium"));
        assert!(level.requires_approval_for_risk("high"));
        assert!(level.requires_approval_for_risk("critical"));
    }

    #[test]
    fn test_autonomy_balanced_asks_high_plus() {
        let level = UserAutonomyLevel::Balanced;
        assert!(!level.requires_approval_for_risk("none"));
        assert!(!level.requires_approval_for_risk("low"));
        assert!(!level.requires_approval_for_risk("medium"));
        assert!(level.requires_approval_for_risk("high"));
        assert!(level.requires_approval_for_risk("critical"));
    }

    #[test]
    fn test_autonomy_autonomous_asks_critical_only() {
        let level = UserAutonomyLevel::Autonomous;
        assert!(!level.requires_approval_for_risk("none"));
        assert!(!level.requires_approval_for_risk("low"));
        assert!(!level.requires_approval_for_risk("medium"));
        assert!(!level.requires_approval_for_risk("high"));
        assert!(level.requires_approval_for_risk("critical"));
    }

    #[test]
    fn test_autonomy_fullauto_asks_nothing() {
        let level = UserAutonomyLevel::FullAuto;
        assert!(!level.requires_approval_for_risk("none"));
        assert!(!level.requires_approval_for_risk("low"));
        assert!(!level.requires_approval_for_risk("medium"));
        assert!(!level.requires_approval_for_risk("high"));
        assert!(!level.requires_approval_for_risk("critical"));
    }

    #[test]
    fn test_autonomy_from_level() {
        assert_eq!(UserAutonomyLevel::from_level(1), Some(UserAutonomyLevel::Supervised));
        assert_eq!(UserAutonomyLevel::from_level(3), Some(UserAutonomyLevel::Balanced));
        assert_eq!(UserAutonomyLevel::from_level(5), Some(UserAutonomyLevel::FullAuto));
        assert_eq!(UserAutonomyLevel::from_level(0), None);
        assert_eq!(UserAutonomyLevel::from_level(6), None);
    }

    #[test]
    fn test_autonomy_display_name() {
        assert!(UserAutonomyLevel::Balanced.display_name().contains("high"));
        assert!(UserAutonomyLevel::FullAuto.display_name().contains("Auto"));
    }

    #[test]
    fn test_profile_backward_compat_no_autonomy_field() {
        // Old profiles without autonomy_level should deserialize with default
        let json = r#"{"user_name":"Test","voice_enabled":false,"onboarding_complete":false,"selected_model":null,"api_key":null,"anthropic_api_key":null,"openai_api_key":null,"google_api_key":null,"theme":null,"auto_approve":false,"default_mode":null,"sounds_enabled":true,"sound_volume":70}"#;
        let p: PersistedProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.autonomy_level, UserAutonomyLevel::Balanced);
    }
}
