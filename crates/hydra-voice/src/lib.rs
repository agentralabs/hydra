//! `hydra-voice` — The Pulse system. Voice I/O for Hydra.
//!
//! Defines the interface for streaming STT/TTS. Actual engines
//! (whisper, piper, etc.) are plugged in at runtime.
//! Voice NEVER blocks the TUI — all operations are async-friendly.

pub mod constants;
pub mod errors;
pub mod microphone;
pub mod native_tts;
pub mod session;
pub mod setup;
pub mod speculative;
pub mod stt;
pub mod system;
pub mod transcribe;
pub mod tts;
pub mod voice_loop;
pub mod wake_word;

// Re-exports for convenience.
pub use errors::VoiceError;
pub use microphone::{MicCapture, MicEvent};
pub use native_tts::TtsEngine;
pub use session::{VoicePresenceState, VoiceSession};
pub use setup::VoiceCapabilities;
pub use wake_word::{WakeWordDetector, WakeWordResult};
pub use speculative::{PredictionCandidate, SpeculativeProcessor, SpeculativeResult};
pub use stt::{CaptureMode, PulseSTT, SttEvent};
pub use system::{VoiceEvent, VoiceSystem};
pub use tts::PulseTTS;
pub use voice_loop::VoiceLoop;
