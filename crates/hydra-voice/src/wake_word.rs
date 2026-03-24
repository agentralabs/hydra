//! O17 Wake Word Detection — lightweight energy-based keyword spotter.
//! Runs always at <1% CPU. Detects energy spike after silence as wake trigger.
//! No ML model required — uses acoustic energy patterns with adaptive noise floor.

use std::collections::VecDeque;
use std::time::Instant;

use crate::constants;

/// Result of processing an audio frame for wake word detection.
#[derive(Debug, Clone, PartialEq)]
pub enum WakeWordResult {
    /// No wake word detected in this frame.
    Nothing,
    /// Wake word detected with given confidence.
    Detected { confidence: f64 },
    /// EC-17.2: Ambient noise too high to reliably detect.
    NoiseFloorTooHigh,
}

/// Lightweight wake word detector using energy-based pattern matching.
/// Detects a sharp energy rise after sustained silence as a voice onset trigger.
pub struct WakeWordDetector {
    /// The wake word being detected (for logging/config).
    keyword: String,
    /// Detection confidence threshold (EC-17.1: default 0.85).
    threshold: f64,
    /// Cooldown between triggers (ms) — prevents rapid re-fire.
    cooldown_ms: u64,
    /// Last time wake word was triggered.
    last_trigger: Option<Instant>,
    /// Whether detection is active.
    active: bool,
    /// Rolling window of RMS energy values for noise floor estimation.
    energy_history: VecDeque<f32>,
    /// Computed adaptive noise floor.
    noise_floor: f32,
    /// Number of consecutive silence frames before this one.
    silence_count: usize,
}

impl WakeWordDetector {
    /// Create a new wake word detector with the given keyword and threshold.
    pub fn new(keyword: &str, threshold: f64) -> Self {
        Self {
            keyword: keyword.to_lowercase(),
            threshold: threshold.clamp(0.5, 1.0),
            cooldown_ms: constants::WAKE_WORD_COOLDOWN_MS,
            last_trigger: None,
            active: true,
            energy_history: VecDeque::with_capacity(constants::NOISE_FLOOR_WINDOW),
            noise_floor: 0.0,
            silence_count: 0,
        }
    }

    /// Create with default settings (keyword "hydra", threshold 0.85).
    pub fn default_detector() -> Self {
        Self::new(constants::WAKE_WORD_DEFAULT, constants::WAKE_WORD_THRESHOLD)
    }

    /// Process one audio frame (chunk of f32 samples at 16kHz mono).
    /// Returns detection result.
    pub fn process_audio_frame(&mut self, frame: &[f32]) -> WakeWordResult {
        if !self.active || frame.is_empty() {
            return WakeWordResult::Nothing;
        }

        // Compute RMS energy
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();

        // Compute noise floor BEFORE adding current frame (so spikes don't pollute it)
        self.noise_floor = if self.energy_history.is_empty() { 0.0 }
            else { self.energy_history.iter().sum::<f32>() / self.energy_history.len() as f32 };

        // EC-17.2: If noise floor is too high, detection unreliable
        if self.noise_floor > constants::SILENCE_THRESHOLD * 10.0 {
            // Still update history (so noise floor can drop when noise subsides)
            if self.energy_history.len() >= constants::NOISE_FLOOR_WINDOW {
                self.energy_history.pop_front();
            }
            self.energy_history.push_back(rms);
            return WakeWordResult::NoiseFloorTooHigh;
        }

        // Check cooldown (EC-17.1)
        if let Some(last) = self.last_trigger {
            if last.elapsed().as_millis() < self.cooldown_ms as u128 {
                // Only add quiet frames to history during cooldown
                if rms < constants::SILENCE_THRESHOLD {
                    if self.energy_history.len() >= constants::NOISE_FLOOR_WINDOW {
                        self.energy_history.pop_front();
                    }
                    self.energy_history.push_back(rms);
                }
                return WakeWordResult::Nothing;
            }
        }

        // Detect voice onset: energy spike after sustained silence
        let spike_threshold = self.noise_floor * constants::NOISE_FLOOR_MULTIPLIER;
        let is_silence = rms < constants::SILENCE_THRESHOLD;
        let is_spike = rms > spike_threshold.max(constants::SILENCE_THRESHOLD * 2.0);

        if is_silence {
            self.silence_count += 1;
            return WakeWordResult::Nothing;
        }

        // Need at least 3 silence frames before spike = voice onset
        if is_spike && self.silence_count >= 3 {
            let confidence = ((rms - self.noise_floor) / rms.max(0.001)) as f64;
            let confidence = confidence.clamp(0.0, 1.0);
            self.silence_count = 0;

            if confidence >= self.threshold {
                self.last_trigger = Some(Instant::now());
                eprintln!("hydra-voice: wake word '{}' detected (conf={confidence:.2})", self.keyword);
                return WakeWordResult::Detected { confidence };
            }
        }

        if !is_silence { self.silence_count = 0; }

        // Only add quiet frames to noise floor history (loud frames are spikes, not floor)
        if is_silence {
            if self.energy_history.len() >= constants::NOISE_FLOOR_WINDOW {
                self.energy_history.pop_front();
            }
            self.energy_history.push_back(rms);
        }

        WakeWordResult::Nothing
    }

    /// EC-17.1: Adjust sensitivity threshold.
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold.clamp(0.5, 1.0);
    }

    /// Enable or disable detection.
    pub fn set_active(&mut self, active: bool) { self.active = active; }

    /// Whether detection is active.
    pub fn is_active(&self) -> bool { self.active }

    /// Get the current keyword.
    pub fn keyword(&self) -> &str { &self.keyword }

    /// Get current noise floor level.
    pub fn noise_floor(&self) -> f32 { self.noise_floor }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_defaults() {
        let d = WakeWordDetector::default_detector();
        assert_eq!(d.keyword(), "hydra");
        assert!(d.is_active());
        assert!((d.threshold - 0.85).abs() < 0.01);
    }

    #[test]
    fn nothing_on_silence() {
        let mut d = WakeWordDetector::default_detector();
        let silence = vec![0.0f32; 3200]; // 200ms at 16kHz
        assert_eq!(d.process_audio_frame(&silence), WakeWordResult::Nothing);
    }

    #[test]
    fn cooldown_prevents_rapid_retrigger() {
        let mut d = WakeWordDetector::new("hydra", 0.3); // Lower threshold
        // Feed true silence to build up silence_count and low noise floor
        let silence = vec![0.0001f32; 3200];
        for _ in 0..5 { d.process_audio_frame(&silence); }
        // Feed loud spike (well above noise floor * 3)
        let loud = vec![0.9f32; 3200];
        let r1 = d.process_audio_frame(&loud);
        assert!(matches!(r1, WakeWordResult::Detected { .. }), "First trigger should detect, got {:?}", r1);
        // Feed silence again then spike — should be in cooldown
        for _ in 0..5 { d.process_audio_frame(&silence); }
        let r2 = d.process_audio_frame(&loud);
        assert_eq!(r2, WakeWordResult::Nothing); // Cooldown active
    }

    #[test]
    fn adjustable_threshold() {
        let mut d = WakeWordDetector::default_detector();
        d.set_threshold(0.95);
        assert!((d.threshold - 0.95).abs() < 0.01);
        d.set_threshold(1.5); // Clamped to 1.0
        assert!((d.threshold - 1.0).abs() < 0.01);
    }

    #[test]
    fn disable_stops_detection() {
        let mut d = WakeWordDetector::default_detector();
        d.set_active(false);
        assert!(!d.is_active());
        let loud = vec![0.8f32; 3200];
        assert_eq!(d.process_audio_frame(&loud), WakeWordResult::Nothing);
    }
}
