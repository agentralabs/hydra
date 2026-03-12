//! Microphone input capture.
//!
//! Provides a platform-agnostic interface for audio capture.
//! The stub implementation always reports no microphone available.
//! Real implementations would use cpal or platform-specific APIs.

use std::sync::atomic::{AtomicBool, Ordering};

/// Microphone input capture.
///
/// Gracefully degrades when no audio capture device is available.
pub struct MicrophoneInput {
    running: AtomicBool,
    sample_rate: u32,
}

impl MicrophoneInput {
    /// Create a new microphone input handle
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            sample_rate: 16000, // 16kHz is standard for speech recognition
        }
    }

    /// Create with a custom sample rate
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self {
            running: AtomicBool::new(false),
            sample_rate,
        }
    }

    /// Check if a microphone is available on the system.
    ///
    /// Stub implementation always returns false.
    /// Real implementation would query cpal/platform audio API.
    pub fn is_available(&self) -> bool {
        // Stub: no microphone detection without cpal
        tracing::debug!("MicrophoneInput::is_available() — stub returns false");
        false
    }

    /// Start capturing audio from the microphone.
    ///
    /// Returns an error if no microphone is available.
    pub fn start(&self) -> Result<(), MicrophoneError> {
        if !self.is_available() {
            return Err(MicrophoneError::NoDevice);
        }
        self.running.store(true, Ordering::SeqCst);
        tracing::info!(sample_rate = self.sample_rate, "Microphone capture started");
        Ok(())
    }

    /// Stop capturing audio
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        tracing::debug!("Microphone capture stopped");
    }

    /// Check if currently capturing
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the configured sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Default for MicrophoneInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Microphone-related errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MicrophoneError {
    /// No microphone device found
    NoDevice,
    /// Permission denied for audio capture
    PermissionDenied,
    /// Capture error
    CaptureError(String),
}

impl MicrophoneError {
    pub fn user_message(&self) -> &str {
        match self {
            Self::NoDevice => "No microphone detected. Voice input unavailable.",
            Self::PermissionDenied => "Microphone permission denied. Check system settings.",
            Self::CaptureError(_) => "Audio capture error. Check microphone connection.",
        }
    }
}

impl std::fmt::Display for MicrophoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDevice => write!(f, "No microphone device found"),
            Self::PermissionDenied => write!(f, "Microphone permission denied"),
            Self::CaptureError(msg) => write!(f, "Microphone capture error: {msg}"),
        }
    }
}

impl std::error::Error for MicrophoneError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_not_available() {
        let mic = MicrophoneInput::new();
        assert!(!mic.is_available());
    }

    #[test]
    fn start_fails_gracefully_when_no_device() {
        let mic = MicrophoneInput::new();
        let result = mic.start();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MicrophoneError::NoDevice));
        assert!(!mic.is_running());
    }

    #[test]
    fn default_sample_rate() {
        let mic = MicrophoneInput::new();
        assert_eq!(mic.sample_rate(), 16000);
    }

    #[test]
    fn custom_sample_rate() {
        let mic = MicrophoneInput::with_sample_rate(44100);
        assert_eq!(mic.sample_rate(), 44100);
    }

    #[test]
    fn stop_is_idempotent() {
        let mic = MicrophoneInput::new();
        mic.stop(); // Should not panic even when not running
        assert!(!mic.is_running());
    }
}
