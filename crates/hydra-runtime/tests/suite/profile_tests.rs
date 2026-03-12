use hydra_runtime::profile::{InterfaceMode, Theme, UserPreferences, UserProfile};
use std::sync::{Arc, Barrier};

#[test]
fn test_profile_create_default() {
    let profile = UserProfile::default();
    assert!(profile.name.is_none());
    assert!(!profile.onboarding_complete);
    assert_eq!(profile.preferences.language, "en");
}

#[test]
fn test_profile_save_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.json");

    let mut profile = UserProfile::default();
    profile.set_name("Alice");
    profile.onboarding_complete = true;
    profile.preferences.voice_enabled = true;
    profile.preferences.theme = Theme::Dark;

    profile.save_to(&path).unwrap();

    let loaded = UserProfile::load_from(&path).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("Alice"));
    assert!(loaded.onboarding_complete);
    assert!(loaded.preferences.voice_enabled);
    assert_eq!(loaded.preferences.theme, Theme::Dark);
}

#[test]
fn test_profile_set_name() {
    let mut profile = UserProfile::default();
    assert!(profile.name.is_none());
    profile.set_name("Bob");
    assert_eq!(profile.name.as_deref(), Some("Bob"));
}

#[test]
fn test_profile_is_first_run() {
    let mut profile = UserProfile::default();
    assert!(profile.is_first_run());
    profile.onboarding_complete = true;
    assert!(!profile.is_first_run());
}

#[test]
fn test_preferences_defaults() {
    let prefs = UserPreferences::default();
    assert!(!prefs.voice_enabled);
    assert!(prefs.sounds_enabled);
    assert!((prefs.sound_volume - 0.7).abs() < f32::EPSILON);
    assert_eq!(prefs.default_mode, InterfaceMode::Companion);
    assert_eq!(prefs.theme, Theme::System);
    assert!(!prefs.wake_word_enabled);
    assert!(!prefs.auto_approve_low_risk);
    assert_eq!(prefs.language, "en");
}

#[test]
fn test_preferences_update() {
    let mut prefs = UserPreferences::default();
    prefs.voice_enabled = true;
    prefs.sounds_enabled = false;
    prefs.sound_volume = 0.3;
    prefs.default_mode = InterfaceMode::Immersive;
    prefs.theme = Theme::Light;
    prefs.wake_word_enabled = true;
    prefs.auto_approve_low_risk = true;
    prefs.language = "ja".into();

    assert!(prefs.voice_enabled);
    assert!(!prefs.sounds_enabled);
    assert!((prefs.sound_volume - 0.3).abs() < f32::EPSILON);
    assert_eq!(prefs.default_mode, InterfaceMode::Immersive);
    assert_eq!(prefs.theme, Theme::Light);
    assert!(prefs.wake_word_enabled);
    assert!(prefs.auto_approve_low_risk);
    assert_eq!(prefs.language, "ja");
}

#[test]
fn test_greeting_with_name() {
    let mut profile = UserProfile::default();
    profile.set_name("Charlie");
    assert_eq!(profile.get_greeting(), "Hi Charlie!");
}

#[test]
fn test_greeting_without_name() {
    let profile = UserProfile::default();
    assert_eq!(profile.get_greeting(), "Hi there!");

    // Empty name should also produce generic greeting
    let mut profile2 = UserProfile::default();
    profile2.name = Some(String::new());
    assert_eq!(profile2.get_greeting(), "Hi there!");
}

#[test]
fn test_profile_migration() {
    // Simulate loading a profile saved by an older version (missing new fields).
    // serde should fill in defaults for missing fields.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old_profile.json");

    let old_json = r#"{
        "name": "Legacy",
        "created_at": "2025-01-01T00:00:00Z",
        "onboarding_complete": true,
        "preferences": {
            "voice_enabled": false,
            "sounds_enabled": true,
            "sound_volume": 0.5,
            "default_mode": "companion",
            "theme": "dark",
            "wake_word_enabled": false,
            "auto_approve_low_risk": false,
            "language": "en"
        }
    }"#;

    std::fs::write(&path, old_json).unwrap();
    let loaded = UserProfile::load_from(&path).unwrap();
    assert_eq!(loaded.name.as_deref(), Some("Legacy"));
    assert!(loaded.onboarding_complete);
    assert_eq!(loaded.preferences.theme, Theme::Dark);
}

#[test]
fn test_profile_concurrent_access() {
    // Two threads write then read the same profile file — last writer wins,
    // but neither should panic or corrupt the file.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("concurrent.json");
    let path1 = path.clone();
    let path2 = path.clone();

    let barrier = Arc::new(Barrier::new(2));
    let b1 = barrier.clone();
    let b2 = barrier.clone();

    let t1 = std::thread::spawn(move || {
        let mut p = UserProfile::default();
        p.set_name("Thread1");
        b1.wait();
        p.save_to(&path1).unwrap();
    });

    let t2 = std::thread::spawn(move || {
        let mut p = UserProfile::default();
        p.set_name("Thread2");
        b2.wait();
        p.save_to(&path2).unwrap();
    });

    t1.join().unwrap();
    t2.join().unwrap();

    // File should be valid JSON with one of the two names
    let loaded = UserProfile::load_from(&path).unwrap();
    let name = loaded.name.unwrap();
    assert!(name == "Thread1" || name == "Thread2");
}
