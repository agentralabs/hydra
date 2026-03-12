use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use hydra_runtime::profile::UserProfile;

use crate::state::AppState;

/// Route definitions for user profile management.
///
/// These define the API surface — actual Axum handler implementations
/// will be wired in when the server routes are registered.
pub struct ProfileRoutes;

impl ProfileRoutes {
    /// GET: retrieve the current user profile
    pub fn get_profile() -> &'static str {
        "/api/profile"
    }

    /// PUT: update the user profile
    pub fn update_profile() -> &'static str {
        "/api/profile"
    }

    /// PUT: set the user's display name
    pub fn set_name() -> &'static str {
        "/api/profile/name"
    }

    /// POST: enable voice mode
    pub fn enable_voice() -> &'static str {
        "/api/profile/voice/enable"
    }

    /// POST: mark onboarding as complete
    pub fn complete_onboarding() -> &'static str {
        "/api/profile/onboarding/complete"
    }

    /// GET: retrieve a contextual greeting
    pub fn get_greeting() -> &'static str {
        "/api/profile/greeting"
    }

    /// GET: check if this is the user's first run
    pub fn is_first_run() -> &'static str {
        "/api/profile/is-first-run"
    }
}

// ═══════════════════════════════════════════════════════════
// REQUEST / RESPONSE TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub voice_enabled: Option<bool>,
    pub sounds_enabled: Option<bool>,
    pub sound_volume: Option<f32>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetNameRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct GreetingResponse {
    pub greeting: String,
}

#[derive(Debug, Serialize)]
pub struct FirstRunResponse {
    pub first_run: bool,
}

// ═══════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════

fn load_profile(state: &AppState) -> Result<UserProfile, (StatusCode, String)> {
    UserProfile::load_from(&state.profile_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load profile: {e}"),
        )
    })
}

fn save_profile(state: &AppState, profile: &UserProfile) -> Result<(), (StatusCode, String)> {
    profile.save_to(&state.profile_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save profile: {e}"),
        )
    })
}

/// GET /api/profile — load and return the user profile as JSON
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let profile = load_profile(&state)?;
    Ok(Json(profile))
}

/// PUT /api/profile — update profile fields and save
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let mut profile = load_profile(&state)?;

    if let Some(name) = req.name {
        profile.set_name(&name);
    }
    if let Some(voice) = req.voice_enabled {
        profile.preferences.voice_enabled = voice;
    }
    if let Some(sounds) = req.sounds_enabled {
        profile.preferences.sounds_enabled = sounds;
    }
    if let Some(vol) = req.sound_volume {
        profile.preferences.sound_volume = vol.clamp(0.0, 1.0);
    }
    if let Some(lang) = req.language {
        profile.preferences.language = lang;
    }

    save_profile(&state, &profile)?;
    Ok(Json(profile))
}

/// PUT /api/profile/name — set the user's display name
pub async fn set_name(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetNameRequest>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let mut profile = load_profile(&state)?;
    profile.set_name(&req.name);
    save_profile(&state, &profile)?;
    Ok(Json(profile))
}

/// POST /api/profile/voice/enable — enable voice in preferences
pub async fn enable_voice(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let mut profile = load_profile(&state)?;
    profile.preferences.voice_enabled = true;
    save_profile(&state, &profile)?;
    Ok(Json(profile))
}

/// POST /api/profile/onboarding/complete — mark onboarding done
pub async fn complete_onboarding(
    State(state): State<Arc<AppState>>,
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let mut profile = load_profile(&state)?;
    profile.onboarding_complete = true;
    save_profile(&state, &profile)?;
    Ok(Json(profile))
}

/// GET /api/profile/greeting — return a personalized greeting
pub async fn get_greeting(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GreetingResponse>, (StatusCode, String)> {
    let profile = load_profile(&state)?;
    Ok(Json(GreetingResponse {
        greeting: profile.get_greeting(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_profile_path() {
        assert_eq!(ProfileRoutes::get_profile(), "/api/profile");
    }

    #[test]
    fn test_update_profile_path() {
        assert_eq!(ProfileRoutes::update_profile(), "/api/profile");
    }

    #[test]
    fn test_set_name_path() {
        assert_eq!(ProfileRoutes::set_name(), "/api/profile/name");
    }

    #[test]
    fn test_enable_voice_path() {
        assert_eq!(ProfileRoutes::enable_voice(), "/api/profile/voice/enable");
    }

    #[test]
    fn test_complete_onboarding_path() {
        assert_eq!(ProfileRoutes::complete_onboarding(), "/api/profile/onboarding/complete");
    }

    #[test]
    fn test_get_greeting_path() {
        assert_eq!(ProfileRoutes::get_greeting(), "/api/profile/greeting");
    }

    #[test]
    fn test_is_first_run_path() {
        assert_eq!(ProfileRoutes::is_first_run(), "/api/profile/is-first-run");
    }

    #[test]
    fn test_update_profile_request_deserialization() {
        let json = serde_json::json!({
            "name": "Alice",
            "voice_enabled": true,
            "sound_volume": 0.8
        });
        let req: UpdateProfileRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.name, Some("Alice".into()));
        assert_eq!(req.voice_enabled, Some(true));
        assert_eq!(req.sound_volume, Some(0.8));
    }

    #[test]
    fn test_update_profile_request_empty() {
        let json = serde_json::json!({});
        let req: UpdateProfileRequest = serde_json::from_value(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.voice_enabled.is_none());
    }

    #[test]
    fn test_set_name_request_deserialization() {
        let json = serde_json::json!({"name": "Bob"});
        let req: SetNameRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.name, "Bob");
    }

    #[test]
    fn test_greeting_response_serialization() {
        let resp = GreetingResponse { greeting: "Hello!".into() };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["greeting"], "Hello!");
    }

    #[test]
    fn test_first_run_response_serialization() {
        let resp = FirstRunResponse { first_run: true };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["first_run"], true);
    }
}

/// GET /api/profile/is-first-run — return whether this is the first run
pub async fn is_first_run(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FirstRunResponse>, (StatusCode, String)> {
    let profile = load_profile(&state)?;
    Ok(Json(FirstRunResponse {
        first_run: profile.is_first_run(),
    }))
}
