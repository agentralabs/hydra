//! SurfaceClass — what a connected device can do.
//! Hydra adapts its output to the surface's capabilities.

use serde::{Deserialize, Serialize};

/// The class of a connected surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceClass {
    /// Full desktop TUI — keyboard, large display, full capabilities.
    DesktopTui,
    /// Mobile device — touch, small screen, notifications.
    Mobile,
    /// Wearable audio — voice in/out, no screen (glasses, earbuds).
    WearableAudio,
    /// Wearable display — small overlay display (AR glasses).
    WearableDisplay,
    /// API client — programmatic, structured output only.
    ApiClient,
    /// Remote terminal — SSH or similar, text-only.
    RemoteTerminal,
    /// Unknown surface — profile on first connect.
    Unknown,
}

impl SurfaceClass {
    /// The output mode best suited to this surface.
    pub fn preferred_output(&self) -> OutputMode {
        match self {
            Self::DesktopTui => OutputMode::FullCockpit,
            Self::Mobile => OutputMode::CompanionView,
            Self::WearableAudio => OutputMode::VoiceOnly,
            Self::WearableDisplay => OutputMode::MinimalOverlay,
            Self::ApiClient => OutputMode::StructuredJson,
            Self::RemoteTerminal => OutputMode::TextStream,
            Self::Unknown => OutputMode::TextStream,
        }
    }

    /// Whether this surface can receive voice output.
    pub fn supports_voice(&self) -> bool {
        matches!(
            self,
            Self::WearableAudio | Self::WearableDisplay | Self::Mobile | Self::DesktopTui
        )
    }

    /// Whether this surface can display rich content.
    pub fn supports_rich_display(&self) -> bool {
        matches!(self, Self::DesktopTui | Self::Mobile)
    }
}

/// The output rendering mode for a surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputMode {
    /// Full cockpit — everything (desktop only).
    FullCockpit,
    /// Companion view — briefings, tasks, signals (mobile).
    CompanionView,
    /// Voice only — no screen output (wearable audio).
    VoiceOnly,
    /// Minimal overlay — key info only (AR glasses).
    MinimalOverlay,
    /// Structured JSON — for API clients.
    StructuredJson,
    /// Plain text stream — remote terminal.
    TextStream,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_prefers_full_cockpit() {
        assert_eq!(
            SurfaceClass::DesktopTui.preferred_output(),
            OutputMode::FullCockpit
        );
    }

    #[test]
    fn wearable_prefers_voice() {
        assert_eq!(
            SurfaceClass::WearableAudio.preferred_output(),
            OutputMode::VoiceOnly
        );
    }

    #[test]
    fn api_client_prefers_json() {
        assert_eq!(
            SurfaceClass::ApiClient.preferred_output(),
            OutputMode::StructuredJson
        );
    }

    #[test]
    fn wearable_audio_supports_voice() {
        assert!(SurfaceClass::WearableAudio.supports_voice());
        assert!(!SurfaceClass::ApiClient.supports_voice());
    }

    #[test]
    fn remote_terminal_prefers_text_stream() {
        assert_eq!(
            SurfaceClass::RemoteTerminal.preferred_output(),
            OutputMode::TextStream
        );
    }

    #[test]
    fn wearable_display_prefers_minimal_overlay() {
        assert_eq!(
            SurfaceClass::WearableDisplay.preferred_output(),
            OutputMode::MinimalOverlay
        );
    }

    #[test]
    fn unknown_prefers_text_stream() {
        assert_eq!(
            SurfaceClass::Unknown.preferred_output(),
            OutputMode::TextStream
        );
    }
}
