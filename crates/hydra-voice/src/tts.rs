//! PulseTTS — streaming text-to-speech processor.
//!
//! Queues sentences for TTS output. Supports interruption (barge-in).
//! The actual TTS engine is plugged in at runtime.

use crate::constants;
use crate::errors::VoiceError;

/// Streaming TTS processor.
#[derive(Debug, Clone)]
pub struct PulseTTS {
    /// Queue of sentences waiting to be spoken.
    queue: Vec<String>,
    /// Whether TTS is currently speaking.
    speaking: bool,
    /// The sentence currently being spoken (if any).
    current_sentence: Option<String>,
}

impl PulseTTS {
    /// Create a new TTS processor.
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            speaking: false,
            current_sentence: None,
        }
    }

    /// Feed a sentence to be spoken. Queues it if already speaking.
    pub fn feed_sentence(&mut self, sentence: String) -> Result<(), VoiceError> {
        if self.queue.len() >= constants::MAX_TTS_QUEUE {
            return Err(VoiceError::TtsQueueFull {
                capacity: constants::MAX_TTS_QUEUE,
            });
        }
        self.queue.push(sentence);

        // If not currently speaking, start the next sentence.
        if !self.speaking {
            self.advance();
        }

        Ok(())
    }

    /// Return whether TTS is currently speaking.
    pub fn is_speaking(&self) -> bool {
        self.speaking
    }

    /// Interrupt the current speech (barge-in). Clears the queue.
    pub fn interrupt(&mut self) {
        self.speaking = false;
        self.current_sentence = None;
        self.queue.clear();
    }

    /// Return the number of sentences in the queue.
    pub fn queue_depth(&self) -> usize {
        self.queue.len()
    }

    /// Return the currently speaking sentence (if any).
    pub fn current_sentence(&self) -> Option<&str> {
        self.current_sentence.as_deref()
    }

    /// Mark the current sentence as finished and advance to next.
    pub fn finish_current(&mut self) {
        self.speaking = false;
        self.current_sentence = None;
        self.advance();
    }

    /// Advance to the next queued sentence.
    fn advance(&mut self) {
        if let Some(sentence) = self.queue.first().cloned() {
            self.queue.remove(0);
            self.current_sentence = Some(sentence);
            self.speaking = true;
        }
    }

    /// Return whether the queue is empty and not speaking.
    pub fn is_idle(&self) -> bool {
        !self.speaking && self.queue.is_empty()
    }
}

impl Default for PulseTTS {
    fn default() -> Self {
        Self::new()
    }
}
