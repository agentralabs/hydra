//! All constants for hydra-voice.
//! No magic numbers anywhere else in this crate.

/// Audio chunk size in milliseconds.
pub const CHUNK_SIZE_MS: u64 = 200;

/// RMS amplitude below which audio is considered silence.
pub const SILENCE_THRESHOLD: f32 = 0.01;

/// RMS amplitude above which barge-in is detected during TTS playback.
pub const BARGE_IN_THRESHOLD: f32 = 0.3;

/// Minimum partial transcript length (chars) before speculative matching.
pub const MIN_PARTIAL_LENGTH: usize = 3;

/// Confidence threshold for speculative matching (spec: 0.75).
pub const SPECULATIVE_MATCH_THRESHOLD: f64 = 0.75;

/// Maximum number of sentences queued for TTS.
pub const MAX_TTS_QUEUE: usize = 16;

/// Number of consecutive silence chunks before a silence event is emitted.
pub const SILENCE_CHUNK_COUNT: usize = 5;

/// Maximum number of prediction candidates.
pub const MAX_PREDICTIONS: usize = 32;

/// Duration tracking granularity (ms).
pub const DURATION_GRANULARITY_MS: u64 = 200;

// ── Voice Presence (O17) ──

/// Wake word detection confidence threshold (EC-17.1: adjustable).
pub const WAKE_WORD_THRESHOLD: f64 = 0.85;
/// Cooldown after wake word trigger (ms) — prevents rapid re-trigger.
pub const WAKE_WORD_COOLDOWN_MS: u64 = 2000;
/// Default wake word.
pub const WAKE_WORD_DEFAULT: &str = "hydra";
/// Seconds of silence before returning to dormant.
pub const SESSION_TIMEOUT_SECS: u64 = 3;
/// Max words for voice responses (shorter than text).
pub const VOICE_MAX_RESPONSE_WORDS: usize = 100;
/// Rolling window size for adaptive noise floor (EC-17.2).
pub const NOISE_FLOOR_WINDOW: usize = 50;
/// Energy must exceed noise floor by this multiplier (EC-17.2).
pub const NOISE_FLOOR_MULTIPLIER: f32 = 3.0;
/// Max queued interrupted messages (EC-17.3).
pub const INTERRUPTED_MSG_MAX: usize = 5;
