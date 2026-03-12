//! Audio player with trait-based backend for platform-agnostic playback.
//!
//! Uses a `StubBackend` by default that logs instead of playing audio.
//! Real backends (rodio, cpal, CoreAudio) can be swapped in later.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

use super::sounds::{SoundConfig, SoundEffect};

// ═══════════════════════════════════════════════════════════
// AUDIO BACKEND TRAIT
// ═══════════════════════════════════════════════════════════

/// Trait for audio playback backends.
/// Implementations handle platform-specific audio output.
pub trait AudioBackend: Send + Sync {
    /// Play a sound effect once
    fn play(&self, effect: SoundEffect, volume: f32) -> Result<(), AudioError>;

    /// Start looping a sound effect (e.g., ambient hum while listening)
    fn play_loop(&self, effect: SoundEffect, volume: f32) -> Result<(), AudioError>;

    /// Stop any looping sound
    fn stop_loop(&self) -> Result<(), AudioError>;

    /// Play raw audio bytes at the given sample rate
    fn play_bytes(&self, bytes: &[u8], sample_rate: u32, volume: f32) -> Result<(), AudioError>;

    /// Check if the backend is functional (audio device available)
    fn is_available(&self) -> bool;

    /// Backend name for diagnostics
    fn name(&self) -> &str;
}

// ═══════════════════════════════════════════════════════════
// STUB BACKEND (logs instead of playing)
// ═══════════════════════════════════════════════════════════

/// Stub backend that logs audio operations without producing sound.
/// Used when no real audio backend is available or during testing.
pub struct StubBackend {
    looping: AtomicBool,
}

impl StubBackend {
    pub fn new() -> Self {
        Self {
            looping: AtomicBool::new(false),
        }
    }
}

impl Default for StubBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for StubBackend {
    fn play(&self, effect: SoundEffect, volume: f32) -> Result<(), AudioError> {
        tracing::debug!(
            backend = "stub",
            effect = ?effect,
            volume = volume,
            "Audio play (stub): {} at volume {:.2}",
            effect.description(),
            volume,
        );
        Ok(())
    }

    fn play_loop(&self, effect: SoundEffect, volume: f32) -> Result<(), AudioError> {
        tracing::debug!(
            backend = "stub",
            effect = ?effect,
            volume = volume,
            "Audio loop start (stub): {} at volume {:.2}",
            effect.description(),
            volume,
        );
        self.looping.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop_loop(&self) -> Result<(), AudioError> {
        tracing::debug!(backend = "stub", "Audio loop stop (stub)");
        self.looping.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn play_bytes(&self, bytes: &[u8], sample_rate: u32, volume: f32) -> Result<(), AudioError> {
        tracing::debug!(
            backend = "stub",
            bytes_len = bytes.len(),
            sample_rate = sample_rate,
            volume = volume,
            "Audio play_bytes (stub): {} bytes at {}Hz, volume {:.2}",
            bytes.len(),
            sample_rate,
            volume,
        );
        Ok(())
    }

    fn is_available(&self) -> bool {
        // Stub is always "available" — it just doesn't produce sound
        true
    }

    fn name(&self) -> &str {
        "stub"
    }
}

// ═══════════════════════════════════════════════════════════
// AUDIO ERROR
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// No audio device available
    NoDevice,
    /// Backend-specific playback error
    PlaybackFailed(String),
    /// Audio system is disabled
    Disabled,
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDevice => write!(f, "No audio output device available"),
            Self::PlaybackFailed(msg) => write!(f, "Audio playback failed: {msg}"),
            Self::Disabled => write!(f, "Audio system is disabled"),
        }
    }
}

impl std::error::Error for AudioError {}

// ═══════════════════════════════════════════════════════════
// AUDIO PLAYER
// ═══════════════════════════════════════════════════════════

/// Main audio player that delegates to an `AudioBackend`.
/// Manages volume, enable/disable state, and sound config.
pub struct AudioPlayer {
    backend: Arc<Mutex<Box<dyn AudioBackend>>>,
    config: Mutex<SoundConfig>,
}

impl AudioPlayer {
    /// Create a new AudioPlayer with the given backend
    pub fn new(backend: Box<dyn AudioBackend>) -> Self {
        Self {
            backend: Arc::new(Mutex::new(backend)),
            config: Mutex::new(SoundConfig::new()),
        }
    }

    /// Create a new AudioPlayer with the stub backend
    pub fn with_stub() -> Self {
        Self::new(Box::new(StubBackend::new()))
    }

    /// Play a sound effect, respecting config (enabled, muted, volume)
    pub fn play(&self, effect: SoundEffect) -> Result<(), AudioError> {
        let config = self.config.lock();
        if !config.is_playable() {
            return Ok(()); // Silently skip — not an error
        }
        let volume = config.volume;
        drop(config);

        self.backend.lock().play(effect, volume)
    }

    /// Start looping a sound effect (e.g., listening hum)
    pub fn play_loop(&self, effect: SoundEffect) -> Result<(), AudioError> {
        let config = self.config.lock();
        if !config.is_playable() {
            return Ok(());
        }
        let volume = config.volume;
        drop(config);

        self.backend.lock().play_loop(effect, volume)
    }

    /// Stop any looping sound
    pub fn stop_loop(&self) -> Result<(), AudioError> {
        self.backend.lock().stop_loop()
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        let mut config = self.config.lock();
        config.volume = volume.clamp(0.0, 1.0);
    }

    /// Get the current volume
    pub fn volume(&self) -> f32 {
        self.config.lock().volume
    }

    /// Enable or disable audio
    pub fn set_enabled(&self, enabled: bool) {
        let mut config = self.config.lock();
        config.enabled = enabled;
    }

    /// Check if audio is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.lock().enabled
    }

    /// Play raw audio bytes
    pub fn play_bytes(&self, bytes: &[u8], sample_rate: u32) -> Result<(), AudioError> {
        let config = self.config.lock();
        if !config.is_playable() {
            return Ok(());
        }
        let volume = config.volume;
        drop(config);

        self.backend.lock().play_bytes(bytes, sample_rate, volume)
    }

    /// Check if the audio backend is available
    pub fn is_available(&self) -> bool {
        self.backend.lock().is_available()
    }

    /// Get the backend name
    pub fn backend_name(&self) -> String {
        self.backend.lock().name().to_string()
    }

    /// Get a reference to the current sound config
    pub fn config(&self) -> SoundConfig {
        self.config.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_backend_is_available() {
        let stub = StubBackend::new();
        assert!(stub.is_available());
        assert_eq!(stub.name(), "stub");
    }

    #[test]
    fn stub_backend_play_succeeds() {
        let stub = StubBackend::new();
        assert!(stub.play(SoundEffect::Wake, 0.5).is_ok());
        assert!(stub.play(SoundEffect::Done, 1.0).is_ok());
        assert!(stub.play(SoundEffect::Error, 0.0).is_ok());
    }

    #[test]
    fn stub_backend_loop_lifecycle() {
        let stub = StubBackend::new();
        assert!(stub.play_loop(SoundEffect::Listening, 0.5).is_ok());
        assert!(stub.looping.load(Ordering::SeqCst));
        assert!(stub.stop_loop().is_ok());
        assert!(!stub.looping.load(Ordering::SeqCst));
    }

    #[test]
    fn player_with_stub() {
        let player = AudioPlayer::with_stub();
        assert!(player.is_available());
        assert_eq!(player.backend_name(), "stub");
    }

    #[test]
    fn player_disabled_skips_play() {
        let player = AudioPlayer::with_stub();
        player.set_enabled(false);
        // Should succeed silently (not produce an error)
        assert!(player.play(SoundEffect::Wake).is_ok());
    }
}
