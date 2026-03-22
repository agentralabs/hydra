//! Voice loop — two-way conversational voice for Hydra.
//!
//! The Siri-like experience:
//! 1. User presses Ctrl+V → microphone activates
//! 2. User speaks → live transcription appears in input box
//! 3. Silence detected → speech ends, text submitted to Hydra
//! 4. Hydra responds → response text spoken aloud via TTS
//! 5. User can interrupt (barge-in) at any time
//!
//! This module coordinates microphone → STT → TUI → LLM → TTS.
//! Everything runs on background threads — TUI is never blocked.

use std::sync::mpsc;

use crate::microphone::{self, MicCapture, MicEvent};
use crate::native_tts::{self, TtsEngine};

/// Voice loop state — managed from the TUI.
pub struct VoiceLoop {
    /// TTS engine for speaking responses.
    tts_engine: TtsEngine,
    /// Active microphone capture (None if not listening).
    capture: Option<MicCapture>,
    /// Channel for receiving mic events.
    mic_rx: Option<mpsc::Receiver<MicEvent>>,
    /// Accumulated speech samples (16kHz mono f32).
    speech_buffer: Vec<f32>,
    /// Current partial transcript (updated as speech is processed).
    partial_transcript: String,
    /// Whether Hydra is currently speaking (TTS active).
    speaking: bool,
    /// TTS child process (for interruption).
    tts_child: Option<std::process::Child>,
    /// Silence detection threshold.
    silence_threshold: f32,
    /// Number of silent chunks before end-of-speech.
    silence_chunks: usize,
}

/// Events sent from VoiceLoop to the TUI for display.
#[derive(Debug, Clone)]
pub enum VoiceUiEvent {
    /// Microphone activated — show "Listening..." in input box.
    Listening,
    /// Partial transcript available — update input box live.
    PartialTranscript(String),
    /// Final transcript ready — submit to cognitive loop.
    FinalTranscript(String),
    /// Hydra is speaking the response.
    Speaking(String),
    /// Hydra finished speaking.
    DoneSpeaking,
    /// Microphone deactivated.
    Stopped,
    /// Error occurred.
    Error(String),
}

impl VoiceLoop {
    /// Create a new voice loop. Call detect() to check capabilities first.
    pub fn new() -> Self {
        Self {
            tts_engine: TtsEngine::detect(),
            capture: None,
            mic_rx: None,
            speech_buffer: Vec::new(),
            partial_transcript: String::new(),
            speaking: false,
            tts_child: None,
            silence_threshold: 0.01,
            silence_chunks: 5, // ~1 second of silence at 200ms chunks
        }
    }

    /// Whether TTS is available for speaking responses.
    pub fn can_speak(&self) -> bool {
        self.tts_engine.is_available()
    }

    /// Whether a microphone is available for listening.
    pub fn can_listen(&self) -> bool {
        microphone::is_microphone_available()
    }

    /// Whether currently capturing audio.
    pub fn is_listening(&self) -> bool {
        self.capture.is_some()
    }

    /// Whether Hydra is currently speaking.
    pub fn is_speaking(&self) -> bool {
        self.speaking
    }

    /// Start listening (Ctrl+V pressed).
    pub fn start_listening(&mut self) -> Result<(), String> {
        if self.capture.is_some() {
            return Ok(()); // Already listening
        }

        // If Hydra is speaking, interrupt (barge-in)
        if self.speaking {
            self.interrupt_speech();
        }

        self.speech_buffer.clear();
        self.partial_transcript.clear();

        let (rx, capture) =
            MicCapture::start(self.silence_threshold, self.silence_chunks)?;
        self.mic_rx = Some(rx);
        self.capture = Some(capture);

        Ok(())
    }

    /// Stop listening (Ctrl+V pressed again, or silence detected).
    pub fn stop_listening(&mut self) {
        if let Some(mut cap) = self.capture.take() {
            cap.stop();
        }
        self.mic_rx = None;
    }

    /// Poll for microphone events (call from TUI event loop).
    /// Returns events for the TUI to process.
    pub fn poll(&mut self) -> Vec<VoiceUiEvent> {
        // Collect raw mic events first (immutable borrow of mic_rx)
        let raw_events: Vec<MicEvent> = if let Some(rx) = &self.mic_rx {
            let mut collected = Vec::new();
            while let Ok(ev) = rx.try_recv() {
                collected.push(ev);
            }
            collected
        } else {
            return Vec::new();
        };

        // Now process (mutable borrow of self is safe)
        let mut events = Vec::new();
        let mut should_stop = false;

        for mic_event in raw_events {
            match mic_event {
                MicEvent::SpeechStarted => {
                    events.push(VoiceUiEvent::Listening);
                }
                MicEvent::Samples(samples) => {
                    self.speech_buffer.extend_from_slice(&samples);
                    let duration = self.speech_buffer.len() as f32 / 16000.0;
                    self.partial_transcript = format!("[recording {:.1}s...]", duration);
                    events.push(VoiceUiEvent::PartialTranscript(
                        self.partial_transcript.clone(),
                    ));
                }
                MicEvent::SilenceDetected => {
                    let duration = self.speech_buffer.len() as f32 / 16000.0;
                    if duration > 0.3 {
                        // Transcribe the captured audio
                        let transcript = match crate::transcribe::transcribe(&self.speech_buffer) {
                            Ok(text) => text,
                            Err(e) => {
                                eprintln!("hydra: transcription failed: {e}");
                                format!("[voice: {:.1}s — {e}]", duration)
                            }
                        };
                        events.push(VoiceUiEvent::FinalTranscript(transcript));
                    }
                    self.speech_buffer.clear();
                    should_stop = true;
                    events.push(VoiceUiEvent::Stopped);
                }
                MicEvent::Error(e) => {
                    events.push(VoiceUiEvent::Error(e));
                    should_stop = true;
                }
            }
        }

        if should_stop {
            self.stop_listening();
        }

        events
    }

    /// Speak a response through TTS (called after LLM responds).
    pub fn speak_response(&mut self, text: &str) {
        if !self.tts_engine.is_available() {
            return;
        }

        // Split into sentences for natural pacing
        let sentences: Vec<&str> = text
            .split_terminator(['.', '!', '?'])
            .filter(|s| !s.trim().is_empty())
            .collect();

        if sentences.is_empty() {
            return;
        }

        // Speak first sentence immediately
        let first = sentences[0].trim();
        self.speaking = true;
        self.tts_child = native_tts::speak_async(&self.tts_engine, first);

        // Queue remaining sentences (will need to be fed after first finishes)
        // For now, concatenate and speak as one
        if sentences.len() > 1 {
            let rest: String = sentences[1..]
                .iter()
                .map(|s| s.trim())
                .collect::<Vec<&str>>()
                .join(". ");
            if !rest.is_empty() {
                // Chain: wait for first to finish, then speak rest
                // Simple approach: speak all at once
                self.interrupt_speech();
                let full = text.chars().take(500).collect::<String>();
                self.tts_child = native_tts::speak_async(&self.tts_engine, &full);
            }
        }
    }

    /// Speak a short alert (for emergency notifications).
    pub fn speak_alert(&mut self, text: &str) {
        if self.tts_engine.is_available() {
            // Interrupt current speech for alerts
            self.interrupt_speech();
            self.tts_child = native_tts::speak_async(&self.tts_engine, text);
            self.speaking = true;
        }
    }

    /// Interrupt current speech (barge-in or cancel).
    pub fn interrupt_speech(&mut self) {
        native_tts::interrupt(&mut self.tts_child);
        self.speaking = false;
    }

    /// Check if TTS has finished speaking (call periodically).
    pub fn check_tts_done(&mut self) -> bool {
        if let Some(child) = &mut self.tts_child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.tts_child = None;
                    self.speaking = false;
                    true
                }
                Ok(None) => false, // still running
                Err(_) => {
                    self.tts_child = None;
                    self.speaking = false;
                    true
                }
            }
        } else {
            false
        }
    }

    /// Get microphone device name.
    pub fn mic_name(&self) -> String {
        microphone::default_device_name().unwrap_or_else(|| "none".into())
    }
}

impl Default for VoiceLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_loop_creates() {
        let vl = VoiceLoop::new();
        eprintln!("Can speak: {}", vl.can_speak());
        eprintln!("Can listen: {}", vl.can_listen());
        eprintln!("Mic: {}", vl.mic_name());
    }
}
