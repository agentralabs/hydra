//! DeviceProfile — one connected device permanently recorded.
//! Stored in cartography after first connection.
//! Never re-profiled on reconnect.

use crate::surface::{OutputMode, SurfaceClass};
use serde::{Deserialize, Serialize};

/// Capabilities a device has declared.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// Whether the device has a microphone.
    pub has_microphone: bool,
    /// Whether the device has a speaker.
    pub has_speaker: bool,
    /// Whether the device has a display.
    pub has_display: bool,
    /// Display width in pixels (None if no display).
    pub display_width: Option<u32>,
    /// Display height in pixels (None if no display).
    pub display_height: Option<u32>,
    /// Whether the device has touch input.
    pub has_touch: bool,
    /// Whether the device has a camera.
    pub has_camera: bool,
    /// Whether the device has a keyboard.
    pub has_keyboard: bool,
    /// Whether the device is mobile.
    pub is_mobile: bool,
}

impl DeviceCapabilities {
    /// Infer the surface class from the device capabilities.
    pub fn infer_surface_class(&self) -> SurfaceClass {
        if self.has_keyboard
            && self.has_display
            && self.display_width.unwrap_or(0) >= 800
        {
            return SurfaceClass::DesktopTui;
        }
        if self.is_mobile && self.has_display {
            return SurfaceClass::Mobile;
        }
        if self.has_microphone && self.has_speaker && !self.has_display {
            return SurfaceClass::WearableAudio;
        }
        if self.has_microphone
            && self.has_display
            && self.display_width.unwrap_or(0) < 400
        {
            return SurfaceClass::WearableDisplay;
        }
        SurfaceClass::Unknown
    }
}

/// A device that has connected to Hydra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// Unique device identifier.
    pub id: String,
    /// Human-readable device name.
    pub name: String,
    /// The inferred surface class.
    pub surface_class: SurfaceClass,
    /// The output mode for this device.
    pub output_mode: OutputMode,
    /// The device capabilities.
    pub capabilities: DeviceCapabilities,
    /// The hashed auth token (never stored in plain).
    pub auth_token: String,
    /// When the device was first seen.
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// When the device was last seen.
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Total connection count.
    pub connection_count: u64,
    /// Whether the device is trusted.
    pub trusted: bool,
}

impl DeviceProfile {
    /// Create a new device profile from capabilities.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        capabilities: DeviceCapabilities,
        auth_token: impl Into<String>,
    ) -> Self {
        let surface_class = capabilities.infer_surface_class();
        let output_mode = surface_class.preferred_output();
        let now = chrono::Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            surface_class,
            output_mode,
            capabilities,
            auth_token: auth_token.into(),
            first_seen: now,
            last_seen: now,
            connection_count: 1,
            trusted: false,
        }
    }

    /// Record a new connection from this device.
    pub fn record_connection(&mut self) {
        self.connection_count += 1;
        self.last_seen = chrono::Utc::now();
    }

    /// Mark this device as trusted.
    pub fn trust(&mut self) {
        self.trusted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_capabilities_infer_correctly() {
        let caps = DeviceCapabilities {
            has_keyboard: true,
            has_display: true,
            display_width: Some(1440),
            display_height: Some(900),
            has_microphone: true,
            has_speaker: true,
            ..Default::default()
        };
        assert_eq!(caps.infer_surface_class(), SurfaceClass::DesktopTui);
    }

    #[test]
    fn wearable_audio_inferred_from_no_display() {
        let caps = DeviceCapabilities {
            has_microphone: true,
            has_speaker: true,
            has_display: false,
            ..Default::default()
        };
        assert_eq!(caps.infer_surface_class(), SurfaceClass::WearableAudio);
    }

    #[test]
    fn mobile_inferred_correctly() {
        let caps = DeviceCapabilities {
            is_mobile: true,
            has_display: true,
            has_touch: true,
            has_microphone: true,
            display_width: Some(390),
            ..Default::default()
        };
        assert_eq!(caps.infer_surface_class(), SurfaceClass::Mobile);
    }

    #[test]
    fn device_profile_created() {
        let caps = DeviceCapabilities {
            has_keyboard: true,
            has_display: true,
            display_width: Some(1920),
            ..Default::default()
        };
        let profile = DeviceProfile::new("dev-1", "Test Device", caps, "token");
        assert_eq!(profile.surface_class, SurfaceClass::DesktopTui);
        assert_eq!(profile.connection_count, 1);
        assert!(!profile.trusted);
    }
}
