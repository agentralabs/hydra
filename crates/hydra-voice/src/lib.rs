//! `hydra-voice` — The Pulse system. Voice I/O for Hydra.
//!
//! Defines the interface for streaming STT/TTS. Actual engines
//! (whisper, piper, etc.) are plugged in at runtime.
//! Voice NEVER blocks the TUI — all operations are async-friendly.

pub mod constants;
pub mod errors;
pub mod speculative;
pub mod stt;
pub mod system;
pub mod tts;

// Re-exports for convenience.
pub use errors::VoiceError;
pub use speculative::{PredictionCandidate, SpeculativeProcessor, SpeculativeResult};
pub use stt::{CaptureMode, PulseSTT, SttEvent};
pub use system::{VoiceEvent, VoiceSystem};
pub use tts::PulseTTS;
