//! Voice loop — push-to-talk + always-listening (O17) with wake word detection.
//! State machine: Dormant → Listening → Processing → Speaking → Listening/Dormant

use std::sync::mpsc;

use crate::microphone::{self, MicCapture, MicEvent};
use crate::native_tts::{self, TtsEngine};
use crate::session::{VoicePresenceState, VoiceSession};
use crate::wake_word::{WakeWordDetector, WakeWordResult};

pub struct VoiceLoop {
    tts_engine: TtsEngine,
    capture: Option<MicCapture>,
    mic_rx: Option<mpsc::Receiver<MicEvent>>,
    speech_buffer: Vec<f32>,
    partial_transcript: String,
    speaking: bool,
    tts_child: Option<std::process::Child>,
    silence_threshold: f32,
    silence_chunks: usize,
    /// O17: Wake word detector for always-listening mode.
    pub wake_word: WakeWordDetector,
    /// O17: Multi-turn voice session state.
    pub session: VoiceSession,
    /// O17: Whether always-listening mode is active.
    pub always_listening: bool,
}

#[derive(Debug, Clone)]
pub enum VoiceUiEvent {
    Listening,
    PartialTranscript(String),
    FinalTranscript(String),
    Speaking(String),
    DoneSpeaking,
    Stopped,
    Error(String),
    WakeWordDetected,   // O17
    SessionTimeout,     // O17
}

impl VoiceLoop {
    /// Push-to-talk mode (default).
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
            silence_chunks: 5,
            wake_word: WakeWordDetector::default_detector(),
            session: VoiceSession::new(),
            always_listening: false,
        }
    }

    /// O17: Create a voice loop in always-listening mode with wake word detection.
    pub fn new_always_listening() -> Self {
        let mut vl = Self::new();
        vl.always_listening = true;
        eprintln!("hydra-voice: always-listening mode enabled (wake word: '{}')", vl.wake_word.keyword());
        vl
    }

    pub fn can_speak(&self) -> bool { self.tts_engine.is_available() }
    pub fn can_listen(&self) -> bool { microphone::is_microphone_available() }
    pub fn is_listening(&self) -> bool { self.capture.is_some() }
    pub fn is_speaking(&self) -> bool { self.speaking }

    /// Start listening (Ctrl+V or wake word detected).
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

    pub fn stop_listening(&mut self) {
        if let Some(mut cap) = self.capture.take() {
            cap.stop();
        }
        self.mic_rx = None;
    }

    /// Poll mic events. Call from TUI event loop.
    pub fn poll(&mut self) -> Vec<VoiceUiEvent> {
        let raw_events: Vec<MicEvent> = if let Some(rx) = &self.mic_rx {
            let mut collected = Vec::new();
            while let Ok(ev) = rx.try_recv() {
                collected.push(ev);
            }
            collected
        } else {
            return Vec::new();
        };

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

    pub fn speak_alert(&mut self, text: &str) {
        if self.tts_engine.is_available() {
            // Interrupt current speech for alerts
            self.interrupt_speech();
            self.tts_child = native_tts::speak_async(&self.tts_engine, text);
            self.speaking = true;
        }
    }

    pub fn interrupt_speech(&mut self) {
        native_tts::interrupt(&mut self.tts_child);
        self.speaking = false;
    }

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

    pub fn mic_name(&self) -> String {
        microphone::default_device_name().unwrap_or_else(|| "none".into())
    }

    /// O17: Poll the always-listening state machine.
    pub fn poll_presence(&mut self) -> Vec<VoiceUiEvent> {
        if !self.always_listening { return Vec::new(); }
        let mut events = Vec::new();

        // Check session timeout (multi-turn → dormant after silence)
        if self.session.check_timeout() {
            self.session.go_dormant();
            self.stop_listening();
            events.push(VoiceUiEvent::SessionTimeout);
            return events;
        }

        // EC-17.9: Skip wake word detection while speaking (prevents feedback loop)
        if self.speaking { return events; }

        // In dormant mode, feed audio to wake word detector
        if self.session.state == VoicePresenceState::Dormant {
            // Process buffered samples through wake word detector
            if !self.speech_buffer.is_empty() {
                let frame = self.speech_buffer.clone();
                self.speech_buffer.clear();
                match self.wake_word.process_audio_frame(&frame) {
                    WakeWordResult::Detected { confidence: _ } => {
                        self.session.activate();
                        events.push(VoiceUiEvent::WakeWordDetected);
                        // Start full STT listening
                        if let Err(e) = self.start_listening() {
                            eprintln!("hydra-voice: failed to start after wake word: {e}");
                            events.push(VoiceUiEvent::Error(e));
                        }
                    }
                    WakeWordResult::NoiseFloorTooHigh => {
                        // EC-17.2: too noisy — silently continue
                    }
                    WakeWordResult::Nothing => {}
                }
            }
        }

        events
    }

    pub fn notify_speech_complete(&mut self) { self.session.speech_complete(); }
    pub fn notify_response_ready(&mut self) { self.session.response_ready(); }
    pub fn notify_done_speaking(&mut self) { self.session.done_speaking(); }

    /// O17: Handle barge-in (EC-17.3: queue interrupted message).
    pub fn handle_barge_in(&mut self, current_speech: &str) {
        if !current_speech.is_empty() {
            self.session.queue_interrupted(current_speech.to_string());
        }
        self.interrupt_speech();
        self.session.done_speaking(); // Back to Listening
    }

    pub fn presence_state(&self) -> &'static str {
        if self.always_listening {
            self.session.state.label()
        } else if self.is_listening() {
            "listening"
        } else if self.is_speaking() {
            "speaking"
        } else {
            "off"
        }
    }

    pub fn voice_response_directive() -> &'static str {
        "You are responding via VOICE. The user is listening, not reading. \
         Rules: Maximum 2-3 sentences for simple answers. Lead with the answer, \
         then explain if needed. Say what you're DOING, not what you COULD do. \
         Skip technical metadata. Ask for confirmation before destructive actions. \
         Use natural speech patterns, not bullet points."
    }

    pub fn is_voice_active(&self) -> bool {
        self.always_listening && self.session.is_active()
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
        assert!(!vl.always_listening);
        assert_eq!(vl.presence_state(), "off");
    }

    #[test]
    fn always_listening_creates() {
        let vl = VoiceLoop::new_always_listening();
        assert!(vl.always_listening);
        assert_eq!(vl.presence_state(), "dormant");
        assert!(vl.wake_word.is_active());
    }

    #[test]
    fn voice_directive_not_empty() {
        let directive = VoiceLoop::voice_response_directive();
        assert!(directive.contains("VOICE"));
        assert!(directive.len() > 50);
    }

    #[test]
    fn presence_state_labels() {
        let mut vl = VoiceLoop::new_always_listening();
        assert_eq!(vl.presence_state(), "dormant");
        vl.session.activate();
        assert_eq!(vl.presence_state(), "listening");
        vl.session.speech_complete();
        assert_eq!(vl.presence_state(), "processing");
        vl.session.response_ready();
        assert_eq!(vl.presence_state(), "speaking");
    }

    #[test]
    fn is_voice_active_tracks_session() {
        let mut vl = VoiceLoop::new_always_listening();
        assert!(!vl.is_voice_active()); // Dormant
        vl.session.activate();
        assert!(vl.is_voice_active()); // Listening
    }
}
