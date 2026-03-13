//! Pulse integration for voice — instant acknowledgment + progressive TTS.
//!
//! Bridges hydra-pulse tiers into the desktop voice flow so the user
//! never waits in silence after speaking.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use hydra_pulse::{ResponsePredictor, TierSelector, ResponseTier};

/// Acknowledgment phrases — varied so they don't feel robotic.
const ACKS: &[&str] = &[
    "Got it.",
    "Working on that.",
    "Let me think.",
    "On it.",
    "One moment.",
    "Let me check.",
    "Looking into that.",
    "Sure, one sec.",
];

/// Pick a varied ack phrase based on a cheap counter.
fn pick_ack(counter: u64) -> &'static str {
    ACKS[(counter as usize) % ACKS.len()]
}

/// Shared pulse state for the desktop app.
pub struct PulseVoice {
    predictor: ResponsePredictor,
    selector: TierSelector,
    ack_counter: parking_lot::Mutex<u64>,
    /// Flag to cancel TTS playback (e.g. when user speaks again).
    pub tts_cancel: Arc<AtomicBool>,
}

impl PulseVoice {
    pub fn new() -> Self {
        Self {
            predictor: ResponsePredictor::new(256, 4),
            selector: TierSelector::with_defaults(),
            ack_counter: parking_lot::Mutex::new(0),
            tts_cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Fire instant spoken acknowledgment (<100ms perceived latency).
    /// Returns the ack text so the UI can display it as a typing indicator.
    /// Call this BEFORE spawning the cognitive loop.
    pub fn instant_ack(&self, _user_text: &str) -> String {
        let mut c = self.ack_counter.lock();
        *c += 1;
        let phrase = pick_ack(*c);

        // Check prediction cache — if we have a high-confidence hit, skip ack
        let prediction = self.predictor.predict(_user_text);
        if prediction.matched && prediction.confidence >= 0.8 {
            // Predictor has a cached response — use that instead of generic ack
            return prediction.response.unwrap_or_else(|| phrase.to_string());
        }

        phrase.to_string()
    }

    /// Learn from a completed exchange so future similar inputs can be predicted.
    pub fn learn(&self, user_text: &str, response: &str) {
        self.predictor.learn(user_text, response);
    }

    /// Cancel any in-flight TTS (e.g. user interrupted by speaking again).
    pub fn cancel_tts(&self) {
        self.tts_cancel.store(true, Ordering::Relaxed);
    }

    /// Reset the cancel flag before starting new TTS playback.
    pub fn reset_tts_cancel(&self) {
        self.tts_cancel.store(false, Ordering::Relaxed);
    }

    /// Whether to use progressive TTS (speak chunks as they arrive).
    pub fn is_progressive(&self) -> bool {
        self.selector.is_progressive()
    }

    /// Which tier to start with for this request.
    pub fn select_tier(&self, user_text: &str) -> ResponseTier {
        let prediction = self.predictor.predict(user_text);
        self.selector.select(prediction.matched, prediction.confidence, false)
    }
}

impl Clone for PulseVoice {
    fn clone(&self) -> Self {
        Self {
            predictor: ResponsePredictor::new(256, 4),
            selector: TierSelector::with_defaults(),
            ack_counter: parking_lot::Mutex::new(*self.ack_counter.lock()),
            tts_cancel: self.tts_cancel.clone(),
        }
    }
}

/// Speak text through TTS, checking the cancel flag periodically.
/// Returns Ok(true) if fully played, Ok(false) if cancelled.
pub async fn speak_interruptible(
    text: &str, api_key: &str, voice: &str, cancel: Arc<AtomicBool>, volume: u8,
) -> Result<bool, String> {
    if text.is_empty() || api_key.is_empty() { return Ok(false); }
    if cancel.load(Ordering::Relaxed) { return Ok(false); }
    eprintln!("[hydra:tts] OpenAI TTS ({} chars, voice={}, vol={})", text.len(), voice, volume);
    let samples = crate::voice_capture::synthesize_openai_tts(text, api_key, voice).await?;
    eprintln!("[hydra:tts] Got {} samples", samples.len());
    if samples.is_empty() || cancel.load(Ordering::Relaxed) { return Ok(false); }
    let cancel_play = cancel.clone();
    let played = tokio::task::spawn_blocking(move || {
        if cancel_play.load(Ordering::Relaxed) { return false; }
        match crate::voice_capture::play_audio(samples, 24000, volume) {
            Ok(()) => { eprintln!("[hydra:tts] Playback OK"); true }
            Err(e) => { eprintln!("[hydra:tts] Playback FAILED: {}", e); false }
        }
    }).await.unwrap_or(false);
    Ok(played)
}

/// Split text into speakable chunks at sentence boundaries.
/// This allows progressive TTS — start speaking the first sentence
/// while the rest of the response is still generating.
pub fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') && current.trim().len() > 5 {
            sentences.push(current.trim().to_string());
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_varies() {
        let pv = PulseVoice::new();
        let a1 = pv.instant_ack("hello");
        let a2 = pv.instant_ack("hello");
        // They should be different (sequential counter)
        assert_ne!(a1, a2);
    }

    #[test]
    fn sentence_split() {
        let text = "Hello there. How are you? I'm fine!";
        let parts = split_into_sentences(text);
        assert_eq!(parts.len(), 3);
        assert!(parts[0].starts_with("Hello"));
        assert!(parts[1].starts_with("How"));
        assert!(parts[2].starts_with("I'm"));
    }

    #[test]
    fn cancel_flag_works() {
        let pv = PulseVoice::new();
        assert!(!pv.tts_cancel.load(Ordering::Relaxed));
        pv.cancel_tts();
        assert!(pv.tts_cancel.load(Ordering::Relaxed));
        pv.reset_tts_cancel();
        assert!(!pv.tts_cancel.load(Ordering::Relaxed));
    }
}
