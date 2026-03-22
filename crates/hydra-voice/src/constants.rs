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
