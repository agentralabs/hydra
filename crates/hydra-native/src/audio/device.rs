//! Audio device detection.
//!
//! Provides a platform-agnostic interface for detecting audio output devices.
//! The stub implementation returns no devices — real implementations query
//! the system audio API (CoreAudio, ALSA, WASAPI).

use serde::{Deserialize, Serialize};

/// Describes an audio output device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Audio device detection and enumeration
pub struct AudioDevice;

impl AudioDevice {
    /// Check if any audio output device is available.
    ///
    /// Stub implementation always returns false — real implementations
    /// query the platform audio API.
    pub fn is_available() -> bool {
        // Stub: no real device detection without cpal/rodio
        // Real implementation would query:
        //   - macOS: CoreAudio
        //   - Linux: ALSA/PulseAudio
        //   - Windows: WASAPI
        tracing::debug!("AudioDevice::is_available() — stub returns false");
        false
    }

    /// List available audio output devices.
    ///
    /// Stub implementation returns an empty list.
    pub fn list_devices() -> Vec<AudioDeviceInfo> {
        // Stub: returns empty — no device enumeration without cpal
        tracing::debug!("AudioDevice::list_devices() — stub returns empty");
        Vec::new()
    }

    /// Get the default output device, if any.
    pub fn default_device() -> Option<AudioDeviceInfo> {
        // Stub: no default device available
        tracing::debug!("AudioDevice::default_device() — stub returns None");
        None
    }

    /// Get the number of available audio output devices.
    pub fn device_count() -> usize {
        Self::list_devices().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_not_available() {
        // Stub always returns false
        assert!(!AudioDevice::is_available());
    }

    #[test]
    fn stub_list_empty() {
        let devices = AudioDevice::list_devices();
        assert!(devices.is_empty());
    }

    #[test]
    fn stub_no_default() {
        assert!(AudioDevice::default_device().is_none());
    }

    #[test]
    fn stub_count_zero() {
        assert_eq!(AudioDevice::device_count(), 0);
    }

    #[test]
    fn device_info_serialization() {
        let device = AudioDeviceInfo {
            name: "Test Speaker".into(),
            is_default: true,
            sample_rate: 44100,
            channels: 2,
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("Test Speaker"));
        let deserialized: AudioDeviceInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Speaker");
        assert!(deserialized.is_default);
    }
}
