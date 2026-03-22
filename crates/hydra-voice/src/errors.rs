//! Voice system error types.

use thiserror::Error;

/// All errors that can occur within the hydra-voice system.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// Audio capture failed.
    #[error("Audio capture error: {reason}")]
    CaptureError {
        /// What went wrong.
        reason: String,
    },

    /// STT processing error.
    #[error("STT processing error: {reason}")]
    SttError {
        /// What went wrong.
        reason: String,
    },

    /// TTS processing error.
    #[error("TTS processing error: {reason}")]
    TtsError {
        /// What went wrong.
        reason: String,
    },

    /// TTS queue is full.
    #[error("TTS queue full (capacity: {capacity})")]
    TtsQueueFull {
        /// The queue capacity.
        capacity: usize,
    },

    /// Voice system not initialized.
    #[error("Voice system not initialized")]
    NotInitialized,
}
