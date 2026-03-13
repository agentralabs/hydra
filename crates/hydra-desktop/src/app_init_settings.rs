//! Extract initial settings values from persisted profile.

use hydra_native::profile::PersistedProfile;

/// Collected initial settings from profile + environment.
pub struct InitSettings {
    pub theme: String,
    pub voice: bool,
    pub sounds: bool,
    pub volume: String,
    pub auto_approve: bool,
    pub default_mode: String,
    pub model: String,
    pub anthropic_key: String,
    pub openai_key: String,
    pub google_key: String,
    pub memory_capture: String,
    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_to: String,
}

/// Extract initial settings from profile (with env var fallbacks for API keys).
pub fn extract_init_settings(persisted: &Option<PersistedProfile>) -> InitSettings {
    let theme = persisted.as_ref()
        .and_then(|p| p.theme.clone())
        .unwrap_or_else(|| "dark".to_string());
    let voice = persisted.as_ref().map_or(false, |p| p.voice_enabled);
    let sounds = persisted.as_ref().map_or(true, |p| p.sounds_enabled);
    let volume = persisted.as_ref().map_or("70".to_string(), |p| p.sound_volume.to_string());
    let auto_approve = persisted.as_ref().map_or(false, |p| p.auto_approve);
    let default_mode = persisted.as_ref()
        .and_then(|p| p.default_mode.clone())
        .unwrap_or_else(|| "companion".to_string());
    let model = persisted.as_ref()
        .and_then(|p| p.selected_model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-6".to_string());

    let anthropic_key = persisted.as_ref()
        .and_then(|p| p.anthropic_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let openai_key = persisted.as_ref()
        .and_then(|p| p.openai_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-") && !k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let google_key = persisted.as_ref()
        .and_then(|p| p.google_api_key.clone())
        .or_else(|| std::env::var("GOOGLE_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();

    let memory_capture = persisted.as_ref()
        .and_then(|p| p.memory_capture.clone())
        .unwrap_or_else(|| "all".into());
    let smtp_host = persisted.as_ref().and_then(|p| p.smtp_host.clone()).unwrap_or_default();
    let smtp_user = persisted.as_ref().and_then(|p| p.smtp_user.clone()).unwrap_or_default();
    let smtp_password = persisted.as_ref().and_then(|p| p.smtp_password.clone()).unwrap_or_default();
    let smtp_to = persisted.as_ref().and_then(|p| p.smtp_to.clone()).unwrap_or_default();

    InitSettings {
        theme, voice, sounds, volume, auto_approve,
        default_mode, model, anthropic_key, openai_key, google_key, memory_capture,
        smtp_host, smtp_user, smtp_password, smtp_to,
    }
}
