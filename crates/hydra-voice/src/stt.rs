//! PulseSTT — streaming speech-to-text processor.
//!
//! Processes audio chunks and emits STT events.
//! The actual STT engine is plugged in at runtime — this crate
//! defines the interface and processes audio data.

use crate::constants;

/// Events emitted by the STT processor.
#[derive(Debug, Clone, PartialEq)]
pub enum SttEvent {
    /// Silence has been detected for a sustained period.
    SilenceDetected,
    /// A partial transcript is available (not final).
    PartialTranscript {
        /// The partial text.
        text: String,
    },
    /// A final transcript is available.
    FinalTranscript {
        /// The final text.
        text: String,
    },
}

/// Capture mode for audio input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// User presses a key to start/stop capture.
    PushToTalk,
    /// Continuous listening with silence detection.
    AlwaysListening,
}

/// Streaming STT processor.
#[derive(Debug, Clone)]
pub struct PulseSTT {
    /// Capture mode.
    mode: CaptureMode,
    /// Whether capture is currently active.
    capturing: bool,
    /// Consecutive silence chunk count.
    silence_chunks: usize,
    /// Accumulated partial transcript.
    partial_text: String,
    /// Total duration of speech captured (ms).
    speech_duration_ms: u64,
    /// Whether barge-in has been detected.
    barge_in_detected: bool,
    /// Whether TTS is currently playing (for barge-in detection).
    tts_playing: bool,
}

impl PulseSTT {
    /// Create a new STT processor with the given capture mode.
    pub fn new(mode: CaptureMode) -> Self {
        Self {
            mode,
            capturing: false,
            silence_chunks: 0,
            partial_text: String::new(),
            speech_duration_ms: 0,
            barge_in_detected: false,
            tts_playing: false,
        }
    }

    /// Start capturing audio.
    pub fn start_capture(&mut self) {
        self.capturing = true;
        self.silence_chunks = 0;
        self.barge_in_detected = false;
    }

    /// Stop capturing audio.
    pub fn stop_capture(&mut self) {
        self.capturing = false;
    }

    /// Return whether capture is active.
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    /// Set whether TTS is currently playing (for barge-in detection).
    pub fn set_tts_playing(&mut self, playing: bool) {
        self.tts_playing = playing;
    }

    /// Process an audio chunk. Returns any STT events produced.
    ///
    /// Each chunk represents `CHUNK_SIZE_MS` milliseconds of audio.
    /// The audio data is f32 samples normalized to [-1.0, 1.0].
    pub fn process_chunk(&mut self, audio: &[f32]) -> Vec<SttEvent> {
        if !self.capturing {
            return Vec::new();
        }

        let mut events = Vec::new();
        let rms = compute_rms(audio);

        // Check for barge-in (speech during TTS playback).
        if self.tts_playing && rms > constants::BARGE_IN_THRESHOLD {
            self.barge_in_detected = true;
        }

        if rms < constants::SILENCE_THRESHOLD {
            self.silence_chunks += 1;
            if self.silence_chunks >= constants::SILENCE_CHUNK_COUNT {
                events.push(SttEvent::SilenceDetected);
                // If we had a partial transcript, finalize it.
                if !self.partial_text.is_empty() {
                    let text = std::mem::take(&mut self.partial_text);
                    events.push(SttEvent::FinalTranscript { text });
                }
                self.silence_chunks = 0;
            }
        } else {
            self.silence_chunks = 0;
            self.speech_duration_ms += constants::CHUNK_SIZE_MS;

            // Simulate partial transcript growth from audio energy.
            // In production, the actual STT engine callback would set this.
            // This is the interface — real transcription is plugged in.
        }

        events
    }

    /// Feed a partial transcript from an external STT engine.
    pub fn feed_partial(&mut self, text: String) -> Vec<SttEvent> {
        self.partial_text = text.clone();
        vec![SttEvent::PartialTranscript { text }]
    }

    /// Feed a final transcript from an external STT engine.
    pub fn feed_final(&mut self, text: String) -> Vec<SttEvent> {
        self.partial_text.clear();
        vec![SttEvent::FinalTranscript { text }]
    }

    /// Return whether barge-in was detected.
    pub fn barge_in_detected(&self) -> bool {
        self.barge_in_detected
    }

    /// Reset barge-in state.
    pub fn clear_barge_in(&mut self) {
        self.barge_in_detected = false;
    }

    /// Return the total speech duration captured (ms).
    pub fn speech_duration_ms(&self) -> u64 {
        self.speech_duration_ms
    }

    /// Return the current capture mode.
    pub fn mode(&self) -> CaptureMode {
        self.mode
    }

    /// Return the current partial transcript.
    pub fn partial_text(&self) -> &str {
        &self.partial_text
    }
}

/// Compute the RMS (root mean square) of an audio buffer.
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_silence() {
        let silence = vec![0.0f32; 100];
        assert!(compute_rms(&silence) < constants::SILENCE_THRESHOLD);
    }

    #[test]
    fn rms_of_speech() {
        let speech: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0).sin() * 0.5).collect();
        assert!(compute_rms(&speech) > constants::SILENCE_THRESHOLD);
    }
}
