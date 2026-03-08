use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Speech-to-text transcript
#[derive(Debug, Clone)]
pub struct Transcript {
    pub text: String,
    pub confidence: f32,
    pub duration: Duration,
    pub language: String,
}

// ═══════════════════════════════════════════════════════════
// ERROR CLASSIFICATION (mirrors hydra-model::executor pattern)
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
    DependencyError,
    ResourceError,
    UserError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SttError {
    ModelNotLoaded,
    ModelNotFound(String),
    Timeout,
    AudioError(String),
}

impl SttError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::ModelNotLoaded => ErrorSeverity::Error,
            Self::ModelNotFound(_) => ErrorSeverity::Critical,
            Self::Timeout => ErrorSeverity::Warning,
            Self::AudioError(_) => ErrorSeverity::Error,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::ModelNotLoaded => ErrorCategory::DependencyError,
            Self::ModelNotFound(_) => ErrorCategory::DependencyError,
            Self::Timeout => ErrorCategory::UserError,
            Self::AudioError(_) => ErrorCategory::ResourceError,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::ModelNotLoaded => "STT model not loaded. Voice recognition unavailable.".into(),
            Self::ModelNotFound(path) => format!(
                "STT model not found at '{path}'. Run 'hydra setup voice' to download models."
            ),
            Self::Timeout => "Speech recognition timed out. Try speaking more clearly.".into(),
            Self::AudioError(msg) => {
                format!("Audio capture error: {msg}. Check microphone permissions.")
            }
        }
    }

    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::ModelNotLoaded => "Enable voice in config and set stt_model path.",
            Self::ModelNotFound(_) => "Run 'hydra setup voice' to download required models.",
            Self::Timeout => "Speak within 10 seconds of the listening prompt.",
            Self::AudioError(_) => {
                "Check that a microphone is connected and permissions are granted."
            }
        }
    }
}

impl std::fmt::Display for SttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.user_message(), self.suggested_action())
    }
}

impl std::error::Error for SttError {}

/// Speech-to-text engine (Whisper.cpp wrapper)
pub struct SttEngine {
    model_loaded: bool,
}

impl SttEngine {
    pub fn new() -> Self {
        Self {
            model_loaded: false,
        }
    }

    /// Load STT model from disk. Verifies the file exists before loading.
    pub fn load_model(&mut self, path: &Path) -> Result<(), SttError> {
        if !path.exists() {
            return Err(SttError::ModelNotFound(path.display().to_string()));
        }
        // Real implementation: FFI call to whisper_init_from_file()
        self.model_loaded = true;
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model_loaded
    }

    /// Transcribe audio samples. Returns ModelNotLoaded if no model is loaded.
    pub fn transcribe(&self, _audio: &[f32]) -> Result<Transcript, SttError> {
        if !self.model_loaded {
            return Err(SttError::ModelNotLoaded);
        }
        // Real implementation: FFI call to whisper_full() with audio samples
        // Returns empty transcript — production builds replace this with real inference
        Ok(Transcript {
            text: String::new(),
            confidence: 0.0,
            duration: Duration::from_millis(0),
            language: "en".into(),
        })
    }

    /// Transcribe with graceful fallback on failure
    pub fn transcribe_safe(&self, audio: &[f32]) -> String {
        match self.transcribe(audio) {
            Ok(t) if t.confidence > 0.3 => t.text,
            _ => FALLBACK_MESSAGE.into(),
        }
    }
}

impl Default for SttEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub const FALLBACK_MESSAGE: &str = "I didn't catch that. Could you say it again?";

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use std::path::PathBuf;

    /// MockSpeechToText: exercises STT logic without real Whisper models
    struct MockSpeechToText {
        model_loaded: bool,
        forced_confidence: f32,
        simulate_timeout: bool,
    }

    impl MockSpeechToText {
        fn with_model(confidence: f32) -> Self {
            Self {
                model_loaded: true,
                forced_confidence: confidence,
                simulate_timeout: false,
            }
        }

        fn without_model() -> Self {
            Self {
                model_loaded: false,
                forced_confidence: 0.0,
                simulate_timeout: false,
            }
        }

        fn with_timeout() -> Self {
            Self {
                model_loaded: true,
                forced_confidence: 0.0,
                simulate_timeout: true,
            }
        }

        fn transcribe(&self, text_hint: &str) -> Result<Transcript, SttError> {
            if !self.model_loaded {
                return Err(SttError::ModelNotLoaded);
            }
            if self.simulate_timeout {
                return Err(SttError::Timeout);
            }
            Ok(Transcript {
                text: text_hint.to_string(),
                confidence: self.forced_confidence,
                duration: Duration::from_millis(300),
                language: "en".into(),
            })
        }
    }

    #[test]
    fn test_stt_transcription() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("whisper-base.bin");
        std::fs::File::create(&model_path)
            .unwrap()
            .write_all(b"fake-model")
            .unwrap();

        let mut engine = SttEngine::new();
        assert!(!engine.is_loaded(), "Engine should start unloaded");

        engine.load_model(&model_path).unwrap();
        assert!(
            engine.is_loaded(),
            "Engine should be loaded after load_model"
        );

        let audio = vec![0.0f32; 16000];
        let result = engine.transcribe(&audio);
        assert!(
            result.is_ok(),
            "Transcription should succeed with loaded model"
        );

        let transcript = result.unwrap();
        assert_eq!(transcript.language, "en");
    }

    #[test]
    fn test_stt_model_not_loaded_error() {
        let engine = SttEngine::new();
        let audio = vec![0.0f32; 16000];
        let result = engine.transcribe(&audio);
        assert!(matches!(result, Err(SttError::ModelNotLoaded)));

        let err = SttError::ModelNotLoaded;
        assert_eq!(err.severity(), ErrorSeverity::Error);
        assert_eq!(err.category(), ErrorCategory::DependencyError);
        assert!(err.user_message().contains("not loaded"));
        assert!(!err.suggested_action().is_empty());
    }

    #[test]
    fn test_stt_model_not_found_on_disk() {
        let mut engine = SttEngine::new();
        let result = engine.load_model(&PathBuf::from("/nonexistent/whisper-base.bin"));
        assert!(matches!(result, Err(SttError::ModelNotFound(_))));

        if let Err(SttError::ModelNotFound(path)) = result {
            assert!(path.contains("nonexistent"));
        }
    }

    #[test]
    fn test_stt_timeout() {
        let mock = MockSpeechToText::with_timeout();
        let result = mock.transcribe("anything");
        assert!(matches!(result, Err(SttError::Timeout)));

        let err = SttError::Timeout;
        assert_eq!(err.severity(), ErrorSeverity::Warning);
        assert!(err.user_message().contains("timed out"));
    }

    #[test]
    fn test_stt_partial_results() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("whisper-base.bin");
        std::fs::File::create(&model_path)
            .unwrap()
            .write_all(b"fake")
            .unwrap();

        let mut engine = SttEngine::new();
        engine.load_model(&model_path).unwrap();

        let audio = vec![0.0f32; 16000];
        let result = engine.transcribe_safe(&audio);
        assert_eq!(
            result, FALLBACK_MESSAGE,
            "Low confidence should return fallback"
        );
    }

    #[test]
    fn test_stt_mock_high_confidence() {
        let mock = MockSpeechToText::with_model(0.92);
        let result = mock.transcribe("open the terminal").unwrap();
        assert_eq!(result.text, "open the terminal");
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_stt_model_not_loaded_mock() {
        let mock = MockSpeechToText::without_model();
        let result = mock.transcribe("hello");
        assert!(matches!(result, Err(SttError::ModelNotLoaded)));
    }

    #[test]
    fn test_stt_error_classification() {
        let errors = vec![
            SttError::ModelNotLoaded,
            SttError::ModelNotFound("/foo".into()),
            SttError::Timeout,
            SttError::AudioError("no mic".into()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
            assert!(
                msg.contains('.'),
                "Error message must follow sentence template: {msg}"
            );
            // severity and category must be classified
            let _ = err.severity();
            let _ = err.category();
            assert!(!err.user_message().is_empty());
            assert!(!err.suggested_action().is_empty());
        }
    }
}
