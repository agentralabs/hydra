//! Integration tests for the hydra-native audio system.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use hydra_native::audio::{
    AudioBackend, AudioDevice, AudioError, AudioPlayer, SimpleMixer, SoundConfig, SoundEffect,
};

// ═══════════════════════════════════════════════════════════
// TEST HELPERS
// ═══════════════════════════════════════════════════════════

/// Counting backend that tracks how many times each method is called
struct CountingBackend {
    play_count: AtomicU32,
    loop_count: AtomicU32,
    stop_count: AtomicU32,
    bytes_count: AtomicU32,
}

impl CountingBackend {
    fn new() -> Self {
        Self {
            play_count: AtomicU32::new(0),
            loop_count: AtomicU32::new(0),
            stop_count: AtomicU32::new(0),
            bytes_count: AtomicU32::new(0),
        }
    }
}

impl AudioBackend for CountingBackend {
    fn play(&self, _effect: SoundEffect, _volume: f32) -> Result<(), AudioError> {
        self.play_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn play_loop(&self, _effect: SoundEffect, _volume: f32) -> Result<(), AudioError> {
        self.loop_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn stop_loop(&self) -> Result<(), AudioError> {
        self.stop_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn play_bytes(&self, _bytes: &[u8], _sample_rate: u32, _volume: f32) -> Result<(), AudioError> {
        self.bytes_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "counting"
    }
}

/// Backend that simulates no audio device
struct NoDeviceBackend;

impl AudioBackend for NoDeviceBackend {
    fn play(&self, _effect: SoundEffect, _volume: f32) -> Result<(), AudioError> {
        Err(AudioError::NoDevice)
    }

    fn play_loop(&self, _effect: SoundEffect, _volume: f32) -> Result<(), AudioError> {
        Err(AudioError::NoDevice)
    }

    fn stop_loop(&self) -> Result<(), AudioError> {
        Err(AudioError::NoDevice)
    }

    fn play_bytes(&self, _bytes: &[u8], _sample_rate: u32, _volume: f32) -> Result<(), AudioError> {
        Err(AudioError::NoDevice)
    }

    fn is_available(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "no-device"
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_audio_player_create() {
    let player = AudioPlayer::with_stub();
    assert!(player.is_enabled());
    assert!(player.is_available());
    assert_eq!(player.backend_name(), "stub");
}

#[test]
fn test_audio_player_no_device_graceful() {
    // Player with no-device backend should handle errors gracefully
    let player = AudioPlayer::new(Box::new(NoDeviceBackend));
    assert!(!player.is_available());

    // Disable audio — play should succeed silently (skipped)
    player.set_enabled(false);
    assert!(player.play(SoundEffect::Wake).is_ok());

    // Re-enable — play returns the backend error
    player.set_enabled(true);
    let result = player.play(SoundEffect::Wake);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AudioError::NoDevice));
}

#[test]
fn test_play_sound_wake() {
    let player = AudioPlayer::with_stub();
    assert!(player.play(SoundEffect::Wake).is_ok());
}

#[test]
fn test_play_sound_done() {
    let player = AudioPlayer::with_stub();
    assert!(player.play(SoundEffect::Done).is_ok());
}

#[test]
fn test_play_sound_error() {
    let player = AudioPlayer::with_stub();
    assert!(player.play(SoundEffect::Error).is_ok());
}

#[test]
fn test_play_loop_start_stop() {
    let player = AudioPlayer::with_stub();
    assert!(player.play_loop(SoundEffect::Listening).is_ok());
    assert!(player.stop_loop().is_ok());
}

#[test]
fn test_volume_control() {
    let player = AudioPlayer::with_stub();

    // Default volume from SoundConfig
    let default_vol = player.volume();
    assert!((default_vol - 0.6).abs() < f32::EPSILON);

    // Set volume
    player.set_volume(0.8);
    assert!((player.volume() - 0.8).abs() < f32::EPSILON);

    // Clamped high
    player.set_volume(5.0);
    assert!((player.volume() - 1.0).abs() < f32::EPSILON);

    // Clamped low
    player.set_volume(-1.0);
    assert!((player.volume() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn test_enable_disable() {
    let backend = CountingBackend::new();
    let player = AudioPlayer::new(Box::new(backend));

    assert!(player.is_enabled());
    assert!(player.play(SoundEffect::Wake).is_ok());

    // Disable
    player.set_enabled(false);
    assert!(!player.is_enabled());
    // Play should succeed but not reach backend
    assert!(player.play(SoundEffect::Wake).is_ok());

    // Re-enable
    player.set_enabled(true);
    assert!(player.is_enabled());
    assert!(player.play(SoundEffect::Done).is_ok());
}

#[test]
fn test_play_bytes() {
    let player = AudioPlayer::with_stub();
    let fake_audio = vec![0u8; 1024];
    assert!(player.play_bytes(&fake_audio, 44100).is_ok());
}

#[test]
fn test_device_detection() {
    // Stub device detection returns no devices
    assert!(!AudioDevice::is_available());
    assert!(AudioDevice::list_devices().is_empty());
    assert!(AudioDevice::default_device().is_none());
    assert_eq!(AudioDevice::device_count(), 0);
}

#[test]
fn test_concurrent_sounds() {
    // Verify player is Send + Sync safe via Arc
    let player = Arc::new(AudioPlayer::with_stub());
    let mut handles = vec![];

    for i in 0..4 {
        let player_clone = Arc::clone(&player);
        let effect = match i % 4 {
            0 => SoundEffect::Wake,
            1 => SoundEffect::Done,
            2 => SoundEffect::Error,
            _ => SoundEffect::Notification,
        };
        let handle = std::thread::spawn(move || {
            player_clone.play(effect).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // No panics or races — test passes if we get here
    assert!(player.is_available());
}

#[test]
fn test_sound_config_defaults() {
    let config = SoundConfig::new();
    assert!(config.enabled);
    assert!((config.volume - 0.6).abs() < f32::EPSILON);
    assert!(!config.muted);
    assert!(config.is_playable());

    // Default via player
    let player = AudioPlayer::with_stub();
    let player_config = player.config();
    assert!(player_config.enabled);
    assert!((player_config.volume - 0.6).abs() < f32::EPSILON);

    // SimpleMixer defaults
    let mixer = SimpleMixer::new();
    assert!((mixer.master_volume() - 1.0).abs() < f32::EPSILON);
    assert_eq!(mixer.channel_count(), 0);
}
