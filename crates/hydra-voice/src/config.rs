use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════
// TIMEOUT CONSTANTS (from arch spec)
// ═══════════════════════════════════════════════════════════

/// STT timeout after speech ends
pub const STT_COMPLETE_TIMEOUT: Duration = Duration::from_secs(10);
/// TTS must begin within this window
pub const TTS_START_TIMEOUT: Duration = Duration::from_secs(2);
/// TTS total synthesis timeout
pub const TTS_COMPLETE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub wake_word: String,
    pub wake_word_model: Option<PathBuf>,
    pub stt_model: Option<PathBuf>,
    pub tts_model: Option<PathBuf>,
    pub silence_timeout: Duration,
    pub max_listen_duration: Duration,
    pub vad_threshold: f32,
    pub local_only: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wake_word: "hey hydra".into(),
            wake_word_model: None,
            stt_model: None,
            tts_model: None,
            silence_timeout: Duration::from_millis(2000),
            max_listen_duration: Duration::from_secs(30),
            vad_threshold: 0.5,
            local_only: true,
        }
    }
}

impl VoiceConfig {
    /// Check whether required model files exist on disk
    pub fn models_available(&self) -> bool {
        let stt_ok = self.stt_model.as_ref().map_or(false, |p| p.exists());
        let tts_ok = self.tts_model.as_ref().map_or(false, |p| p.exists());
        stt_ok && tts_ok
    }

    /// Check whether model paths are configured (paths set, existence not verified)
    pub fn models_configured(&self) -> bool {
        self.stt_model.is_some() && self.tts_model.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;

    #[test]
    fn test_voice_config_defaults() {
        let cfg = VoiceConfig::default();
        assert!(!cfg.enabled, "Voice must be disabled by default");
        assert_eq!(cfg.wake_word, "hey hydra");
        assert_eq!(cfg.silence_timeout, Duration::from_millis(2000));
        assert_eq!(cfg.max_listen_duration, Duration::from_secs(30));
        assert!((cfg.vad_threshold - 0.5).abs() < f32::EPSILON);
        assert!(cfg.local_only, "local_only must default to true");
    }

    #[test]
    fn test_voice_config_models_available_checks_disk() {
        let mut cfg = VoiceConfig::default();
        assert!(
            !cfg.models_available(),
            "No models available with None paths"
        );

        // Paths set but files don't exist
        cfg.stt_model = Some(PathBuf::from("/nonexistent/whisper-base.bin"));
        cfg.tts_model = Some(PathBuf::from("/nonexistent/piper.onnx"));
        assert!(cfg.models_configured(), "Paths are configured");
        assert!(
            !cfg.models_available(),
            "Non-existent paths must return false"
        );

        // Create real temp files
        let dir = tempfile::tempdir().unwrap();
        let stt_path = dir.path().join("whisper-base.bin");
        let tts_path = dir.path().join("piper.onnx");
        std::fs::File::create(&stt_path)
            .unwrap()
            .write_all(b"fake")
            .unwrap();
        std::fs::File::create(&tts_path)
            .unwrap()
            .write_all(b"fake")
            .unwrap();

        cfg.stt_model = Some(stt_path);
        cfg.tts_model = Some(tts_path);
        assert!(cfg.models_available(), "Existing files must return true");
    }

    #[test]
    fn test_timeout_constants() {
        assert_eq!(STT_COMPLETE_TIMEOUT, Duration::from_secs(10));
        assert_eq!(TTS_START_TIMEOUT, Duration::from_secs(2));
        assert_eq!(TTS_COMPLETE_TIMEOUT, Duration::from_secs(30));
    }
}
