use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub wake_word: String,
    pub tts_enabled: bool,
    pub stt_engine: SttEngine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SttEngine {
    Whisper,
    System,
}

impl VoiceConfig {
    pub fn new() -> Self {
        Self {
            wake_word: "Hey Hydra".to_string(),
            tts_enabled: false,
            stt_engine: SttEngine::Whisper,
        }
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse natural-language voice approval into a boolean.
///
/// Returns `Some(true)` for affirmative phrases, `Some(false)` for
/// negative phrases, and `None` for unrecognized input.
pub fn parse_voice_approval(text: &str) -> Option<bool> {
    let normalized = text.trim().to_lowercase();
    let affirmative = [
        "yes",
        "yeah",
        "yep",
        "go ahead",
        "do it",
        "approved",
        "confirm",
        "sure",
        "ok",
        "okay",
    ];
    let negative = [
        "no",
        "nope",
        "stop",
        "cancel",
        "deny",
        "don't",
        "abort",
        "wait",
    ];

    if affirmative.iter().any(|&a| normalized.contains(a)) {
        Some(true)
    } else if negative.iter().any(|&n| normalized.contains(n)) {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affirmative_phrases() {
        assert_eq!(parse_voice_approval("yes"), Some(true));
        assert_eq!(parse_voice_approval("Go ahead"), Some(true));
        assert_eq!(parse_voice_approval("Sure, do it"), Some(true));
        assert_eq!(parse_voice_approval("  Yeah  "), Some(true));
    }

    #[test]
    fn negative_phrases() {
        assert_eq!(parse_voice_approval("no"), Some(false));
        assert_eq!(parse_voice_approval("Stop"), Some(false));
        assert_eq!(parse_voice_approval("cancel that"), Some(false));
        assert_eq!(parse_voice_approval("Wait"), Some(false));
    }

    #[test]
    fn unrecognized_returns_none() {
        assert_eq!(parse_voice_approval("banana"), None);
        assert_eq!(parse_voice_approval("hmm"), None);
        assert_eq!(parse_voice_approval(""), None);
    }

    #[test]
    fn default_config() {
        let config = VoiceConfig::new();
        assert_eq!(config.wake_word, "Hey Hydra");
        assert!(!config.tts_enabled);
        assert_eq!(config.stt_engine, SttEngine::Whisper);
    }
}
