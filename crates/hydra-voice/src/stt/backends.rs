use std::time::Duration;

use super::{SttError, Transcript};

// ═══════════════════════════════════════════════════════════
// TRAIT-BASED STT ENGINE
// ═══════════════════════════════════════════════════════════

/// Trait for pluggable speech-to-text backends.
/// Allows swapping between Whisper, system STT, cloud APIs, or mocks.
pub trait SttBackend: Send + Sync {
    /// Transcribe audio samples to text
    fn transcribe(&self, audio: &[f32]) -> Result<Transcript, SttError>;

    /// Check if the backend is ready (model loaded, etc.)
    fn is_ready(&self) -> bool;

    /// Backend name for diagnostics
    fn backend_name(&self) -> &str;
}

/// Whisper stub backend — returns a graceful "model not available" error.
/// Real implementation would use whisper-rs FFI.
pub struct WhisperStub;

impl WhisperStub {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WhisperStub {
    fn default() -> Self {
        Self::new()
    }
}

impl SttBackend for WhisperStub {
    fn transcribe(&self, _audio: &[f32]) -> Result<Transcript, SttError> {
        tracing::debug!("WhisperStub::transcribe — model not available, returning error");
        Err(SttError::ModelNotLoaded)
    }

    fn is_ready(&self) -> bool {
        false
    }

    fn backend_name(&self) -> &str {
        "whisper-stub"
    }
}

/// Mock STT engine for testing — returns configurable results.
pub struct MockSttEngine {
    response_text: String,
    confidence: f32,
    should_fail: Option<SttError>,
}

impl MockSttEngine {
    /// Create a mock that returns successful transcriptions
    pub fn with_text(text: impl Into<String>, confidence: f32) -> Self {
        Self {
            response_text: text.into(),
            confidence,
            should_fail: None,
        }
    }

    /// Create a mock that always fails with the given error
    pub fn failing(error: SttError) -> Self {
        Self {
            response_text: String::new(),
            confidence: 0.0,
            should_fail: Some(error),
        }
    }
}

impl SttBackend for MockSttEngine {
    fn transcribe(&self, _audio: &[f32]) -> Result<Transcript, SttError> {
        if let Some(ref err) = self.should_fail {
            return Err(err.clone());
        }
        Ok(Transcript {
            text: self.response_text.clone(),
            confidence: self.confidence,
            duration: Duration::from_millis(300),
            language: "en".into(),
        })
    }

    fn is_ready(&self) -> bool {
        self.should_fail.is_none()
    }

    fn backend_name(&self) -> &str {
        "mock"
    }
}
