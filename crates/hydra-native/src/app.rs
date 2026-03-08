//! Root application component — Dioxus App entry point.
//!
//! When the `desktop` feature is enabled, this renders the full UI.
//! Without it, this module defines the app layout structure for testing.

use serde::{Deserialize, Serialize};

use crate::state::hydra::{CognitivePhase, GlobeState};

/// Application layout sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSection {
    Chat,
    Settings,
}

/// Application view model — the data needed to render the full UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppViewModel {
    /// Current active section
    pub section: AppSection,
    /// Whether connected to the hydra server
    pub connected: bool,
    /// Current globe state
    pub globe_state: GlobeState,
    /// Active cognitive phase (if a run is in progress)
    pub active_phase: Option<CognitivePhase>,
    /// Number of messages
    pub message_count: usize,
    /// Total tokens used
    pub total_tokens: u64,
    /// Error message (if any)
    pub error: Option<String>,
    /// Application version
    pub version: String,
}

impl AppViewModel {
    pub fn from_state(state: &crate::state::hydra::HydraState) -> Self {
        Self {
            section: AppSection::Chat,
            connected: state.connected,
            globe_state: state.globe_state,
            active_phase: state.active_phase(),
            message_count: state.messages.len(),
            total_tokens: state.total_tokens(),
            error: state.error.clone(),
            version: "0.1.0".into(),
        }
    }

    /// Status bar text
    pub fn status_text(&self) -> String {
        if !self.connected {
            return "Disconnected".into();
        }
        if let Some(phase) = &self.active_phase {
            return format!("Running: {}", phase.label());
        }
        if self.total_tokens > 0 {
            format!("Ready | {} tokens used", self.total_tokens)
        } else {
            "Ready".into()
        }
    }

    /// Connection indicator CSS class
    pub fn connection_class(&self) -> &'static str {
        if self.connected {
            "status-connected"
        } else {
            "status-disconnected"
        }
    }
}

/// Window configuration for the native app
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Hydra".into(),
            width: 480,
            height: 720,
            min_width: 380,
            min_height: 500,
            resizable: true,
            decorations: true,
            transparent: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::hydra::HydraState;

    #[test]
    fn test_view_model_from_state() {
        let state = HydraState::with_defaults();
        let vm = AppViewModel::from_state(&state);
        assert_eq!(vm.section, AppSection::Chat);
        assert!(!vm.connected);
        assert_eq!(vm.globe_state, GlobeState::Idle);
        assert_eq!(vm.message_count, 0);
    }

    #[test]
    fn test_status_text_disconnected() {
        let state = HydraState::with_defaults();
        let vm = AppViewModel::from_state(&state);
        assert_eq!(vm.status_text(), "Disconnected");
    }

    #[test]
    fn test_status_text_ready() {
        let mut state = HydraState::with_defaults();
        state.set_connected(true);
        let vm = AppViewModel::from_state(&state);
        assert_eq!(vm.status_text(), "Ready");
    }

    #[test]
    fn test_status_text_with_tokens() {
        let mut state = HydraState::with_defaults();
        state.set_connected(true);
        state.add_hydra_message("test", None, Some(500));
        let vm = AppViewModel::from_state(&state);
        assert_eq!(vm.status_text(), "Ready | 500 tokens used");
    }

    #[test]
    fn test_status_text_running() {
        let mut state = HydraState::with_defaults();
        state.set_connected(true);
        state.handle_run_started("run-1", "test");
        state.handle_step_started("run-1", CognitivePhase::Think);
        let vm = AppViewModel::from_state(&state);
        assert_eq!(vm.status_text(), "Running: Think");
    }

    #[test]
    fn test_connection_class() {
        let vm = AppViewModel {
            section: AppSection::Chat,
            connected: true,
            globe_state: GlobeState::Idle,
            active_phase: None,
            message_count: 0,
            total_tokens: 0,
            error: None,
            version: "0.1.0".into(),
        };
        assert_eq!(vm.connection_class(), "status-connected");
    }

    #[test]
    fn test_window_config_defaults() {
        let config = WindowConfig::default();
        assert_eq!(config.title, "Hydra");
        assert_eq!(config.width, 480);
        assert_eq!(config.height, 720);
        assert!(config.resizable);
    }
}
