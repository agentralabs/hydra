use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Synthesized audio output
#[derive(Debug, Clone)]
pub struct SynthesizedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: Duration,
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
pub enum TtsError {
    ModelNotLoaded,
    ModelNotFound(String),
    EmptyInput,
    Timeout,
    AudioError(String),
}

impl TtsError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::ModelNotLoaded => ErrorSeverity::Error,
            Self::ModelNotFound(_) => ErrorSeverity::Critical,
            Self::EmptyInput => ErrorSeverity::Warning,
            Self::Timeout => ErrorSeverity::Warning,
            Self::AudioError(_) => ErrorSeverity::Error,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::ModelNotLoaded => ErrorCategory::DependencyError,
            Self::ModelNotFound(_) => ErrorCategory::DependencyError,
            Self::EmptyInput => ErrorCategory::UserError,
            Self::Timeout => ErrorCategory::ResourceError,
            Self::AudioError(_) => ErrorCategory::ResourceError,
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::ModelNotLoaded => {
                "TTS model not loaded. Voice output unavailable. Using text display instead.".into()
            }
            Self::ModelNotFound(path) => format!(
                "TTS model not found at '{path}'. Run 'hydra setup voice' to download models."
            ),
            Self::EmptyInput => "Nothing to speak. The response was empty.".into(),
            Self::Timeout => "Speech synthesis timed out. Falling back to text display.".into(),
            Self::AudioError(msg) => format!("Audio output error: {msg}. Check speaker settings."),
        }
    }

    pub fn suggested_action(&self) -> &'static str {
        match self {
            Self::ModelNotLoaded => "Enable voice in config and set tts_model path.",
            Self::ModelNotFound(_) => "Run 'hydra setup voice' to download required models.",
            Self::EmptyInput => "Ensure the response pipeline produces output before TTS.",
            Self::Timeout => "Response will be shown as text instead.",
            Self::AudioError(_) => {
                "Check that speakers are connected and audio permissions are granted."
            }
        }
    }
}

impl std::fmt::Display for TtsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.user_message(), self.suggested_action())
    }
}

impl std::error::Error for TtsError {}

/// Text-to-speech engine (Piper TTS wrapper)
pub struct TtsEngine {
    model_loaded: bool,
    interruptible: bool,
}

impl TtsEngine {
    pub fn new() -> Self {
        Self {
            model_loaded: false,
            interruptible: true,
        }
    }

    /// Load TTS model from disk. Verifies the file exists before loading.
    pub fn load_model(&mut self, path: &Path) -> Result<(), TtsError> {
        if !path.exists() {
            return Err(TtsError::ModelNotFound(path.display().to_string()));
        }
        // Real implementation: FFI call to piper_init()
        self.model_loaded = true;
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model_loaded
    }

    /// Synthesize text to audio. Returns ModelNotLoaded if no model is loaded.
    pub fn synthesize(&self, text: &str) -> Result<SynthesizedAudio, TtsError> {
        if !self.model_loaded {
            return Err(TtsError::ModelNotLoaded);
        }
        if text.is_empty() {
            return Err(TtsError::EmptyInput);
        }
        // Real implementation: FFI call to piper_text_to_audio()
        // Returns synthetic silence — production builds replace with real inference
        let sample_rate = 22050u32;
        let duration_ms = (text.len() * 50) as u64;
        let num_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
        Ok(SynthesizedAudio {
            samples: vec![0.0; num_samples],
            sample_rate,
            duration: Duration::from_millis(duration_ms),
        })
    }

    /// Synthesize with text fallback on failure
    pub fn synthesize_or_text(&self, text: &str) -> Result<SynthesizedAudio, String> {
        match self.synthesize(text) {
            Ok(audio) => Ok(audio),
            Err(_) => Err(text.to_string()),
        }
    }

    pub fn is_interruptible(&self) -> bool {
        self.interruptible
    }
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub use crate::tts_backends::{MockTtsEngine, PiperStub, TtsBackend};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    /// MockTextToSpeech: exercises TTS logic without real Piper models
    struct MockTextToSpeech {
        model_loaded: bool,
        simulate_timeout: bool,
        interrupted: Arc<AtomicBool>,
    }

    impl MockTextToSpeech {
        fn with_model() -> Self {
            Self {
                model_loaded: true,
                simulate_timeout: false,
                interrupted: Arc::new(AtomicBool::new(false)),
            }
        }

        fn with_timeout() -> Self {
            Self {
                model_loaded: true,
                simulate_timeout: true,
                interrupted: Arc::new(AtomicBool::new(false)),
            }
        }

        fn synthesize(&self, text: &str) -> Result<SynthesizedAudio, TtsError> {
            if !self.model_loaded {
                return Err(TtsError::ModelNotLoaded);
            }
            if self.simulate_timeout {
                return Err(TtsError::Timeout);
            }
            if self.interrupted.load(Ordering::SeqCst) {
                return Err(TtsError::AudioError(
                    "Barge-in interrupted synthesis".into(),
                ));
            }
            Ok(SynthesizedAudio {
                samples: vec![0.0f32; text.len() * 100],
                sample_rate: 22050,
                duration: Duration::from_millis((text.len() * 50) as u64),
            })
        }

        fn interrupt(&self) {
            self.interrupted.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_tts_synthesis() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("piper.onnx");
        std::fs::File::create(&model_path)
            .unwrap()
            .write_all(b"fake-model")
            .unwrap();

        let mut engine = TtsEngine::new();
        assert!(!engine.is_loaded(), "Engine should start unloaded");

        engine.load_model(&model_path).unwrap();
        assert!(
            engine.is_loaded(),
            "Engine should be loaded after load_model"
        );

        let result = engine.synthesize("Hello from Hydra");
        assert!(result.is_ok(), "Synthesis should succeed with loaded model");

        let audio = result.unwrap();
        assert_eq!(audio.sample_rate, 22050, "Sample rate should be 22050 Hz");
        assert!(!audio.samples.is_empty(), "Audio samples must not be empty");
    }

    #[test]
    fn test_tts_empty_input_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("piper.onnx");
        std::fs::File::create(&model_path)
            .unwrap()
            .write_all(b"fake")
            .unwrap();

        let mut engine = TtsEngine::new();
        engine.load_model(&model_path).unwrap();

        let result = engine.synthesize("");
        assert!(matches!(result, Err(TtsError::EmptyInput)));
    }

    #[test]
    fn test_tts_model_not_found_on_disk() {
        let mut engine = TtsEngine::new();
        let result = engine.load_model(std::path::Path::new("/nonexistent/piper.onnx"));
        assert!(matches!(result, Err(TtsError::ModelNotFound(_))));

        if let Err(TtsError::ModelNotFound(path)) = result {
            assert!(path.contains("nonexistent"));
        }
    }

    #[test]
    fn test_tts_timeout() {
        let mock = MockTextToSpeech::with_timeout();
        let result = mock.synthesize("Hello");
        assert!(matches!(result, Err(TtsError::Timeout)));

        let err = TtsError::Timeout;
        assert_eq!(err.severity(), ErrorSeverity::Warning);
        assert!(err.user_message().contains("timed out"));
    }

    #[test]
    fn test_tts_interruptible() {
        let engine = TtsEngine::new();
        assert!(
            engine.is_interruptible(),
            "TTS engine must support barge-in interruption"
        );

        let mock = MockTextToSpeech::with_model();
        mock.interrupt();
        let result = mock.synthesize("This should be interrupted");
        assert!(
            result.is_err(),
            "Interrupted synthesis must return an error"
        );
    }

    #[test]
    fn test_tts_fallback_to_text() {
        let engine = TtsEngine::new(); // model not loaded
        let result = engine.synthesize_or_text("respond with text");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "respond with text");
    }

    #[test]
    fn test_tts_model_not_loaded_error() {
        let mock = MockTextToSpeech {
            model_loaded: false,
            simulate_timeout: false,
            interrupted: Arc::new(AtomicBool::new(false)),
        };
        let result = mock.synthesize("hello");
        assert!(matches!(result, Err(TtsError::ModelNotLoaded)));

        let err = TtsError::ModelNotLoaded;
        assert_eq!(err.severity(), ErrorSeverity::Error);
        assert_eq!(err.category(), ErrorCategory::DependencyError);
        assert!(err.user_message().contains("not loaded"));
    }

    #[test]
    fn test_tts_error_classification() {
        let errors = vec![
            TtsError::ModelNotLoaded,
            TtsError::ModelNotFound("/foo".into()),
            TtsError::EmptyInput,
            TtsError::Timeout,
            TtsError::AudioError("no speaker".into()),
        ];
        for err in &errors {
            let msg = format!("{err}");
            assert!(!msg.is_empty());
            assert!(msg.contains('.'));
            let _ = err.severity();
            let _ = err.category();
            assert!(!err.user_message().is_empty());
            assert!(!err.suggested_action().is_empty());
        }
    }

    #[test]
    fn test_tts_audio_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let model_path = dir.path().join("piper.onnx");
        std::fs::File::create(&model_path)
            .unwrap()
            .write_all(b"fake")
            .unwrap();

        let mut engine = TtsEngine::new();
        engine.load_model(&model_path).unwrap();

        let audio = engine.synthesize("test").unwrap();
        // "test" = 4 chars * 50ms = 200ms at 22050 Hz = 4410 samples
        let expected_samples = (22050u64 * 200 / 1000) as usize;
        assert_eq!(
            audio.samples.len(),
            expected_samples,
            "Sample count must match duration * sample_rate"
        );
        assert_eq!(audio.duration, Duration::from_millis(200));
    }
}
