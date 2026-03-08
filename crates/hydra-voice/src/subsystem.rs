use crate::config::VoiceConfig;
use crate::state::{VoiceSession, VoiceState};
use crate::stt::SttEngine;
use crate::tts::TtsEngine;
use crate::wake_word::WakeWordDetector;

use serde::{Deserialize, Serialize};

/// The voice subsystem — owns all voice state
///
/// State ownership:
/// - Owns WakeWordDetector, SttEngine, TtsEngine, VoiceSession
/// - Models loaded once at startup
/// - Session state cleared on silence timeout
pub struct VoiceSubsystem {
    config: VoiceConfig,
    wake: WakeWordDetector,
    stt: SttEngine,
    tts: TtsEngine,
    session: VoiceSession,
}

impl VoiceSubsystem {
    /// Initialize voice subsystem. Missing models degrade gracefully (voice disabled, not crash).
    pub fn init(config: VoiceConfig) -> Result<Self, VoiceInitError> {
        let wake = WakeWordDetector::new(&config.wake_word, config.vad_threshold);
        let mut stt = SttEngine::new();
        let mut tts = TtsEngine::new();

        if config.enabled {
            if let Some(ref path) = config.stt_model {
                if let Err(e) = stt.load_model(path) {
                    tracing::warn!("STT model load failed: {e}. Voice input disabled.");
                }
            }
            if let Some(ref path) = config.tts_model {
                if let Err(e) = tts.load_model(path) {
                    tracing::warn!("TTS model load failed: {e}. Using text fallback.");
                }
            }
        }

        let session = VoiceSession::new();

        Ok(Self {
            config,
            wake,
            stt,
            tts,
            session,
        })
    }

    /// Start voice (wake word detection begins)
    pub fn start(&mut self) {
        if self.config.enabled {
            self.wake.start();
            self.session.transition(VoiceState::Idle);
        }
    }

    /// Stop all voice activity
    pub fn stop(&mut self) {
        self.wake.stop();
        self.session.transition(VoiceState::Disabled);
    }

    /// Current voice state
    pub fn state(&self) -> VoiceState {
        self.session.state()
    }

    /// Is voice enabled and running?
    pub fn is_active(&self) -> bool {
        self.config.enabled && self.wake.is_running()
    }

    /// Check for voice collision (speaking while should be listening)
    pub fn has_collision(&self) -> bool {
        self.session.is_collision()
    }

    /// Handle barge-in: user speaks during TTS output.
    /// Transitions Speaking → Listening and returns true if barge-in occurred.
    pub fn handle_barge_in(&self) -> bool {
        if self.session.state() == VoiceState::Speaking {
            self.session.transition(VoiceState::Listening)
        } else {
            false
        }
    }

    /// STT fallback message when recognition fails
    pub fn stt_fallback() -> &'static str {
        crate::stt::FALLBACK_MESSAGE
    }

    /// Synthesize and play audio, or return the text for display on TTS failure
    pub fn speak_or_display(&self, text: &str) -> Result<(), String> {
        match self.tts.synthesize_or_text(text) {
            Ok(_audio) => {
                self.session.transition(VoiceState::Speaking);
                // Real implementation: sends audio to platform audio output (CoreAudio/ALSA/WASAPI)
                // The audio samples + sample_rate are in the SynthesizedAudio struct
                Ok(())
            }
            Err(fallback_text) => Err(fallback_text),
        }
    }

    pub fn session(&self) -> &VoiceSession {
        &self.session
    }

    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    pub fn stt(&self) -> &SttEngine {
        &self.stt
    }

    pub fn tts(&self) -> &TtsEngine {
        &self.tts
    }
}

// ═══════════════════════════════════════════════════════════
// INIT ERROR (classified with severity + category + user message)
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    ResourceError,
    DependencyError,
}

#[derive(Debug, Clone)]
pub enum VoiceInitError {
    AudioDeviceNotFound,
    ModelLoadFailed(String),
}

impl VoiceInitError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::AudioDeviceNotFound => ErrorSeverity::Warning,
            Self::ModelLoadFailed(_) => ErrorSeverity::Error,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::AudioDeviceNotFound => ErrorCategory::ResourceError,
            Self::ModelLoadFailed(_) => ErrorCategory::DependencyError,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::AudioDeviceNotFound => "No audio device found. Voice will be disabled. Connect a microphone to enable voice.".into(),
            Self::ModelLoadFailed(msg) => format!("Voice model failed to load: {msg}. Voice will use text fallback."),
        }
    }

    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::AudioDeviceNotFound => "Connect a microphone and restart Hydra.",
            Self::ModelLoadFailed(_) => "Run 'hydra setup voice' to download models.",
        }
    }
}

impl std::fmt::Display for VoiceInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.user_message(), self.suggested_action())
    }
}

impl std::error::Error for VoiceInitError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn disabled_config() -> VoiceConfig {
        VoiceConfig {
            enabled: false,
            ..VoiceConfig::default()
        }
    }

    fn enabled_no_models() -> VoiceConfig {
        VoiceConfig {
            enabled: true,
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

    #[test]
    fn test_voice_missing_models() {
        let config = enabled_no_models();
        let result = VoiceSubsystem::init(config);
        assert!(result.is_ok(), "Missing models must not crash voice init");

        let subsystem = result.unwrap();
        assert!(
            !subsystem.stt().is_loaded(),
            "STT should be unloaded when no path given"
        );
        assert!(
            !subsystem.tts().is_loaded(),
            "TTS should be unloaded when no path given"
        );
        assert_eq!(subsystem.state(), VoiceState::Idle);
    }

    #[test]
    fn test_voice_disabled_config() {
        let config = disabled_config();
        let subsystem = VoiceSubsystem::init(config).unwrap();
        assert!(
            !subsystem.is_active(),
            "Disabled config must not activate voice"
        );
    }

    #[test]
    fn test_voice_device_unavailable_error() {
        let err = VoiceInitError::AudioDeviceNotFound;
        assert_eq!(err.severity(), ErrorSeverity::Warning);
        assert_eq!(err.category(), ErrorCategory::ResourceError);
        let msg = err.user_message();
        assert!(msg.contains("microphone") || msg.contains("audio") || msg.contains("device"));
        assert!(!err.suggested_action().is_empty());
    }

    #[test]
    fn test_voice_model_load_failed_error() {
        let err = VoiceInitError::ModelLoadFailed("corrupt onnx file".into());
        assert_eq!(err.severity(), ErrorSeverity::Error);
        assert_eq!(err.category(), ErrorCategory::DependencyError);
        let msg = err.user_message();
        assert!(msg.contains("model") || msg.contains("Model"));
        assert!(!err.suggested_action().is_empty());
    }

    #[test]
    fn test_voice_start_stop() {
        let mut subsystem = VoiceSubsystem::init(disabled_config()).unwrap();
        subsystem.start();
        assert!(
            !subsystem.is_active(),
            "Disabled subsystem must not activate on start()"
        );

        subsystem.stop();
        assert!(
            !subsystem.is_active(),
            "Disabled subsystem must remain inactive after stop()"
        );
    }

    #[test]
    fn test_voice_barge_in_via_subsystem() {
        let subsystem = VoiceSubsystem::init(disabled_config()).unwrap();
        let result = subsystem.handle_barge_in();
        assert!(!result, "Barge-in on non-Speaking state must return false");
    }

    #[test]
    fn test_voice_stt_fallback_message() {
        let fallback = VoiceSubsystem::stt_fallback();
        assert!(fallback.contains("didn't catch") || fallback.contains("say it again"));
    }

    #[test]
    fn test_voice_config_accessible() {
        let config = enabled_no_models();
        let subsystem = VoiceSubsystem::init(config).unwrap();
        assert_eq!(subsystem.config().wake_word, "hey hydra");
        assert!(
            subsystem.config().local_only,
            "local_only must default to true"
        );
    }

    #[test]
    fn test_voice_enabled_start_stop() {
        let config = VoiceConfig {
            enabled: true,
            ..VoiceConfig::default()
        };
        let mut subsystem = VoiceSubsystem::init(config).unwrap();
        subsystem.start();
        assert!(
            subsystem.is_active(),
            "Enabled subsystem must be active after start()"
        );

        subsystem.stop();
        assert!(
            !subsystem.is_active(),
            "Subsystem must be inactive after stop()"
        );
        assert_eq!(subsystem.state(), VoiceState::Disabled);
    }
}
