use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundEffect {
    Wake,         // Soft chime, two gentle tones
    Listening,    // Quiet ambient hum
    Done,         // Satisfying brief ding
    Approval,     // Musical two-tone (friendly doorbell)
    Error,        // Gentle "hmm" (NOT alarming)
    Notification, // Soft ping (barely there)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundConfig {
    pub enabled: bool,
    pub volume: f32, // 0.0 - 1.0
    pub muted: bool,
}

impl SoundConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            volume: 0.6,
            muted: false,
        }
    }

    pub fn with_volume(volume: f32) -> Self {
        Self {
            enabled: true,
            volume: volume.clamp(0.0, 1.0),
            muted: false,
        }
    }

    pub fn is_playable(&self) -> bool {
        self.enabled && !self.muted && self.volume > 0.0
    }
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundEffect {
    pub fn description(&self) -> &str {
        match self {
            SoundEffect::Wake => "Soft chime, two gentle tones",
            SoundEffect::Listening => "Quiet ambient hum",
            SoundEffect::Done => "Satisfying brief ding",
            SoundEffect::Approval => "Musical two-tone, friendly doorbell",
            SoundEffect::Error => "Gentle hmm, not alarming",
            SoundEffect::Notification => "Soft ping, barely there",
        }
    }

    pub fn duration_ms(&self) -> u32 {
        match self {
            SoundEffect::Wake => 400,
            SoundEffect::Listening => 0, // ongoing
            SoundEffect::Done => 200,
            SoundEffect::Approval => 500,
            SoundEffect::Error => 300,
            SoundEffect::Notification => 150,
        }
    }

    /// All Hydra sounds are gentle by design requirement
    pub fn is_gentle(&self) -> bool {
        true
    }

    /// Base frequency and optional second tone frequency in Hz
    pub fn frequency_hz(&self) -> (u32, Option<u32>) {
        match self {
            SoundEffect::Wake => (440, Some(523)),
            SoundEffect::Listening => (220, None),
            SoundEffect::Done => (880, None),
            SoundEffect::Approval => (523, Some(659)),
            SoundEffect::Error => (330, Some(294)),
            SoundEffect::Notification => (1047, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_EFFECTS: [SoundEffect; 6] = [
        SoundEffect::Wake,
        SoundEffect::Listening,
        SoundEffect::Done,
        SoundEffect::Approval,
        SoundEffect::Error,
        SoundEffect::Notification,
    ];

    #[test]
    fn all_effects_are_gentle() {
        for effect in &ALL_EFFECTS {
            assert!(effect.is_gentle(), "{:?} should be gentle", effect);
        }
    }

    #[test]
    fn durations_are_reasonable() {
        for effect in &ALL_EFFECTS {
            let d = effect.duration_ms();
            // Listening is ongoing (0), all others under 1 second
            assert!(d <= 1000, "{:?} duration {} too long", effect, d);
        }
    }

    #[test]
    fn config_playable_logic() {
        let config = SoundConfig::new();
        assert!(config.is_playable());

        let mut muted = SoundConfig::new();
        muted.muted = true;
        assert!(!muted.is_playable());

        let mut disabled = SoundConfig::new();
        disabled.enabled = false;
        assert!(!disabled.is_playable());

        let zero_vol = SoundConfig::with_volume(0.0);
        assert!(!zero_vol.is_playable());
    }

    #[test]
    fn config_defaults() {
        let config = SoundConfig::new();
        assert!(config.enabled);
        assert!((config.volume - 0.6).abs() < f32::EPSILON);
        assert!(!config.muted);
    }

    #[test]
    fn volume_clamped() {
        let high = SoundConfig::with_volume(2.0);
        assert!((high.volume - 1.0).abs() < f32::EPSILON);
        let low = SoundConfig::with_volume(-1.0);
        assert!((low.volume - 0.0).abs() < f32::EPSILON);
    }
}
