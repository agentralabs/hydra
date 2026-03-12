//! Simple audio mixer for combining volume levels.
//!
//! Provides volume mixing, ducking, and master/channel volume control.

/// Simple mixer for combining volume levels across channels.
///
/// Supports a master volume and per-channel volumes. The effective volume
/// for any channel is `master * channel_volume`.
pub struct SimpleMixer {
    master_volume: f32,
    channels: Vec<MixerChannel>,
}

/// A named mixer channel with its own volume
#[derive(Debug, Clone)]
pub struct MixerChannel {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

impl MixerChannel {
    pub fn new(name: impl Into<String>, volume: f32) -> Self {
        Self {
            name: name.into(),
            volume: volume.clamp(0.0, 1.0),
            muted: false,
        }
    }

    /// Effective volume (0.0 if muted)
    pub fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}

impl SimpleMixer {
    /// Create a new mixer with default master volume (1.0)
    pub fn new() -> Self {
        Self {
            master_volume: 1.0,
            channels: Vec::new(),
        }
    }

    /// Create a mixer with a specific master volume
    pub fn with_master_volume(volume: f32) -> Self {
        Self {
            master_volume: volume.clamp(0.0, 1.0),
            channels: Vec::new(),
        }
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Get the master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Add a named channel with a volume level
    pub fn add_channel(&mut self, name: impl Into<String>, volume: f32) {
        self.channels.push(MixerChannel::new(name, volume));
    }

    /// Set volume for a channel by name. Returns false if channel not found.
    pub fn set_channel_volume(&mut self, name: &str, volume: f32) -> bool {
        if let Some(ch) = self.channels.iter_mut().find(|c| c.name == name) {
            ch.volume = volume.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// Mute or unmute a channel. Returns false if channel not found.
    pub fn set_channel_muted(&mut self, name: &str, muted: bool) -> bool {
        if let Some(ch) = self.channels.iter_mut().find(|c| c.name == name) {
            ch.muted = muted;
            true
        } else {
            false
        }
    }

    /// Get the effective volume for a channel (master * channel)
    pub fn effective_volume(&self, name: &str) -> Option<f32> {
        self.channels
            .iter()
            .find(|c| c.name == name)
            .map(|ch| self.master_volume * ch.effective_volume())
    }

    /// Mix a raw volume value through the master volume
    pub fn mix(&self, volume: f32) -> f32 {
        (self.master_volume * volume).clamp(0.0, 1.0)
    }

    /// Get the number of channels
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// List all channel names
    pub fn channel_names(&self) -> Vec<&str> {
        self.channels.iter().map(|c| c.name.as_str()).collect()
    }
}

impl Default for SimpleMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_master_volume() {
        let mixer = SimpleMixer::new();
        assert!((mixer.master_volume() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn master_volume_clamped() {
        let mut mixer = SimpleMixer::new();
        mixer.set_master_volume(2.0);
        assert!((mixer.master_volume() - 1.0).abs() < f32::EPSILON);
        mixer.set_master_volume(-1.0);
        assert!((mixer.master_volume() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn channel_effective_volume() {
        let mut mixer = SimpleMixer::with_master_volume(0.5);
        mixer.add_channel("effects", 0.8);
        let eff = mixer.effective_volume("effects").unwrap();
        assert!((eff - 0.4).abs() < f32::EPSILON); // 0.5 * 0.8 = 0.4
    }

    #[test]
    fn channel_mute() {
        let mut mixer = SimpleMixer::new();
        mixer.add_channel("music", 0.7);
        assert!(mixer.set_channel_muted("music", true));
        assert!((mixer.effective_volume("music").unwrap() - 0.0).abs() < f32::EPSILON);

        assert!(mixer.set_channel_muted("music", false));
        assert!((mixer.effective_volume("music").unwrap() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_channel_returns_none() {
        let mixer = SimpleMixer::new();
        assert!(mixer.effective_volume("nonexistent").is_none());
    }

    #[test]
    fn mix_raw_value() {
        let mixer = SimpleMixer::with_master_volume(0.5);
        let mixed = mixer.mix(0.6);
        assert!((mixed - 0.3).abs() < f32::EPSILON);
    }
}
