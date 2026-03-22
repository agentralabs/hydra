//! VoiceSystem — unified Pulse coordinator.
//!
//! Combines STT, TTS, and speculative processing into a single
//! async-friendly system. Voice NEVER blocks the TUI.

use crate::errors::VoiceError;
use crate::speculative::{PredictionCandidate, SpeculativeProcessor, SpeculativeResult};
use crate::stt::{CaptureMode, PulseSTT, SttEvent};
use crate::tts::PulseTTS;

/// Events emitted by the voice system.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceEvent {
    /// User interrupted TTS by speaking (barge-in).
    BargeIn,
    /// A transcript is ready for processing.
    TranscriptReady {
        /// The transcript text.
        text: String,
    },
    /// A speculative match was found.
    SpeculativeMatch {
        /// The matched intent.
        intent: String,
    },
}

/// The unified voice system coordinator.
#[derive(Debug, Clone)]
pub struct VoiceSystem {
    /// STT processor.
    stt: PulseSTT,
    /// TTS processor.
    tts: PulseTTS,
    /// Speculative processor.
    speculative: SpeculativeProcessor,
    /// Whether the voice system is initialized.
    initialized: bool,
}

impl VoiceSystem {
    /// Create a new voice system with the given capture mode.
    pub fn new(mode: CaptureMode) -> Self {
        Self {
            stt: PulseSTT::new(mode),
            tts: PulseTTS::new(),
            speculative: SpeculativeProcessor::new(),
            initialized: false,
        }
    }

    /// Initialize the voice system.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Start audio capture.
    pub fn start_capture(&mut self) -> Result<(), VoiceError> {
        if !self.initialized {
            return Err(VoiceError::NotInitialized);
        }
        self.stt.start_capture();
        Ok(())
    }

    /// Stop audio capture.
    pub fn stop_capture(&mut self) {
        self.stt.stop_capture();
    }

    /// Process an audio chunk and return any voice events.
    pub fn process_audio(&mut self, audio: &[f32]) -> Vec<VoiceEvent> {
        let mut events = Vec::new();

        // Update STT about TTS state.
        self.stt.set_tts_playing(self.tts.is_speaking());

        // Process audio through STT.
        let stt_events = self.stt.process_chunk(audio);

        // Check for barge-in.
        if self.stt.barge_in_detected() {
            self.tts.interrupt();
            self.stt.clear_barge_in();
            events.push(VoiceEvent::BargeIn);
        }

        // Process STT events.
        for stt_event in stt_events {
            match stt_event {
                SttEvent::PartialTranscript { ref text } => {
                    let result = self.speculative.check_partial(text);
                    if let SpeculativeResult::Match { intent, .. } = result {
                        events.push(VoiceEvent::SpeculativeMatch { intent });
                    }
                }
                SttEvent::FinalTranscript { ref text } => {
                    let _ = self.speculative.validate_final(text);
                    events.push(VoiceEvent::TranscriptReady { text: text.clone() });
                }
                SttEvent::SilenceDetected => {
                    // Silence is handled implicitly by FinalTranscript.
                }
            }
        }

        events
    }

    /// Feed a partial transcript from an external STT engine.
    pub fn feed_partial(&mut self, text: String) -> Vec<VoiceEvent> {
        let mut events = Vec::new();
        let stt_events = self.stt.feed_partial(text);

        for stt_event in stt_events {
            if let SttEvent::PartialTranscript { ref text } = stt_event {
                let result = self.speculative.check_partial(text);
                if let SpeculativeResult::Match { intent, .. } = result {
                    events.push(VoiceEvent::SpeculativeMatch { intent });
                }
            }
        }

        events
    }

    /// Feed a final transcript from an external STT engine.
    pub fn feed_final(&mut self, text: String) -> Vec<VoiceEvent> {
        let mut events = Vec::new();
        let stt_events = self.stt.feed_final(text.clone());

        for stt_event in stt_events {
            if let SttEvent::FinalTranscript { ref text } = stt_event {
                let _ = self.speculative.validate_final(text);
                events.push(VoiceEvent::TranscriptReady { text: text.clone() });
            }
        }

        events
    }

    /// Speak a sentence through TTS.
    pub fn speak_sentence(&mut self, sentence: String) -> Result<(), VoiceError> {
        self.tts.feed_sentence(sentence)
    }

    /// Update speculative prediction candidates.
    pub fn update_predictions(&mut self, candidates: Vec<PredictionCandidate>) {
        self.speculative.update_predictions(candidates);
    }

    /// Return whether TTS is currently speaking.
    pub fn is_speaking(&self) -> bool {
        self.tts.is_speaking()
    }

    /// Return whether audio capture is active.
    pub fn is_capturing(&self) -> bool {
        self.stt.is_capturing()
    }

    /// Return whether the voice system is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Return a reference to the STT processor.
    pub fn stt(&self) -> &PulseSTT {
        &self.stt
    }

    /// Return a reference to the TTS processor.
    pub fn tts(&self) -> &PulseTTS {
        &self.tts
    }

    /// Return a reference to the speculative processor.
    pub fn speculative(&self) -> &SpeculativeProcessor {
        &self.speculative
    }
}
