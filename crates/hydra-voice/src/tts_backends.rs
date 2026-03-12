use crate::tts::TtsError;

/// Trait for pluggable text-to-speech backends.
/// Allows swapping between Piper, system TTS, cloud APIs, or mocks.
pub trait TtsBackend: Send + Sync {
    /// Synthesize text to raw audio bytes (PCM signed 16-bit LE)
    fn synthesize(&self, text: &str) -> Result<Vec<u8>, TtsError>;

    /// Check if the backend is ready (model loaded, etc.)
    fn is_ready(&self) -> bool;

    /// Backend name for diagnostics
    fn backend_name(&self) -> &str;
}

/// Piper stub backend — returns a graceful "model not available" error.
/// Real implementation would use piper-rs FFI.
pub struct PiperStub;

impl PiperStub {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PiperStub {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsBackend for PiperStub {
    fn synthesize(&self, _text: &str) -> Result<Vec<u8>, TtsError> {
        tracing::debug!("PiperStub::synthesize — model not available, returning error");
        Err(TtsError::ModelNotLoaded)
    }

    fn is_ready(&self) -> bool {
        false
    }

    fn backend_name(&self) -> &str {
        "piper-stub"
    }
}

/// Mock TTS engine for testing — returns configurable audio output.
pub struct MockTtsEngine {
    should_fail: Option<TtsError>,
    /// Bytes per character of input text (for generating fake audio)
    bytes_per_char: usize,
}

impl MockTtsEngine {
    /// Create a mock that returns synthetic audio bytes
    pub fn new() -> Self {
        Self {
            should_fail: None,
            bytes_per_char: 100,
        }
    }

    /// Create a mock that always fails
    pub fn failing(error: TtsError) -> Self {
        Self {
            should_fail: Some(error),
            bytes_per_char: 0,
        }
    }
}

impl Default for MockTtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsBackend for MockTtsEngine {
    fn synthesize(&self, text: &str) -> Result<Vec<u8>, TtsError> {
        if let Some(ref err) = self.should_fail {
            return Err(err.clone());
        }
        if text.is_empty() {
            return Err(TtsError::EmptyInput);
        }
        // Generate fake PCM data proportional to text length
        let num_bytes = text.len() * self.bytes_per_char;
        Ok(vec![0u8; num_bytes])
    }

    fn is_ready(&self) -> bool {
        self.should_fail.is_none()
    }

    fn backend_name(&self) -> &str {
        "mock"
    }
}
