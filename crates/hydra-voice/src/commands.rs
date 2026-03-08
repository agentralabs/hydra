//! Voice command parser — maps transcribed speech to Hydra actions.

use serde::{Deserialize, Serialize};

/// A parsed voice command
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub action: VoiceAction,
    pub raw_text: String,
    pub confidence: ConfidenceLevel,
}

/// Recognized voice actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceAction {
    /// "Hydra approve" / "yes" / "go ahead"
    Approve,
    /// "Hydra deny" / "no" / "reject"
    Deny,
    /// "Hydra stop" / "stop" / "cancel" / "kill"
    Stop,
    /// "Hydra explain" / "what are you doing" / "explain"
    Explain,
    /// "Hydra status" / "what's happening"
    Status,
    /// "Hydra undo" / "undo that" / "revert"
    Undo,
    /// Free-form intent — not a recognized command, pass to cognitive loop
    FreeForm { intent: String },
}

/// Confidence level of command recognition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

/// Parse transcribed text into a voice command.
/// Returns `VoiceAction::FreeForm` if no command pattern matches.
pub fn parse_command(text: &str) -> VoiceCommand {
    let normalized = text.trim().to_lowercase();
    let words: Vec<&str> = normalized.split_whitespace().collect();

    // Strip "hydra" prefix if present
    let command_words: &[&str] = if words.first() == Some(&"hydra") {
        &words[1..]
    } else if words.len() >= 2 && words[0] == "hey" && words[1] == "hydra" {
        &words[2..]
    } else {
        &words
    };

    let command_str = command_words.join(" ");

    let (action, confidence) = match command_str.as_str() {
        // Approve patterns
        "approve" | "yes" | "go ahead" | "proceed" | "do it" | "confirmed" | "accept" => {
            (VoiceAction::Approve, ConfidenceLevel::High)
        }
        "okay" | "sure" | "fine" | "yep" | "yeah" => {
            (VoiceAction::Approve, ConfidenceLevel::Medium)
        }

        // Deny patterns
        "deny" | "no" | "reject" | "decline" | "don't" | "refuse" => {
            (VoiceAction::Deny, ConfidenceLevel::High)
        }
        "nah" | "nope" | "not now" | "skip" => (VoiceAction::Deny, ConfidenceLevel::Medium),

        // Stop patterns
        "stop" | "cancel" | "kill" | "halt" | "abort" | "emergency stop" => {
            (VoiceAction::Stop, ConfidenceLevel::High)
        }
        "stop everything" | "kill it" | "shut down" | "shut it down" => {
            (VoiceAction::Stop, ConfidenceLevel::High)
        }

        // Explain patterns
        "explain" | "what are you doing" | "explain that" | "why" => {
            (VoiceAction::Explain, ConfidenceLevel::High)
        }
        "what's happening" | "what happened" | "tell me more" => {
            (VoiceAction::Explain, ConfidenceLevel::Medium)
        }

        // Status patterns
        "status" | "how's it going" | "progress" | "update" => {
            (VoiceAction::Status, ConfidenceLevel::High)
        }

        // Undo patterns
        "undo" | "undo that" | "revert" | "roll back" | "take it back" => {
            (VoiceAction::Undo, ConfidenceLevel::High)
        }

        // No match — free-form intent
        _ => {
            // Check for partial matches
            if command_str.contains("approve") || command_str.contains("go ahead") {
                (VoiceAction::Approve, ConfidenceLevel::Low)
            } else if command_str.contains("stop") || command_str.contains("cancel") {
                (VoiceAction::Stop, ConfidenceLevel::Low)
            } else if command_str.contains("explain") || command_str.contains("what") {
                (VoiceAction::Explain, ConfidenceLevel::Low)
            } else {
                (
                    VoiceAction::FreeForm {
                        intent: text.trim().to_string(),
                    },
                    ConfidenceLevel::High,
                )
            }
        }
    };

    VoiceCommand {
        action,
        raw_text: text.trim().to_string(),
        confidence,
    }
}

/// Check if a command is safe to execute without confirmation.
/// High-confidence approve/deny/status are safe. Stop requires high confidence.
pub fn is_safe_to_execute(command: &VoiceCommand) -> bool {
    match (&command.action, command.confidence) {
        (VoiceAction::Approve, ConfidenceLevel::High | ConfidenceLevel::Medium) => true,
        (VoiceAction::Deny, ConfidenceLevel::High | ConfidenceLevel::Medium) => true,
        (VoiceAction::Status, _) => true,
        (VoiceAction::Explain, _) => true,
        (VoiceAction::Stop, ConfidenceLevel::High) => true,
        (VoiceAction::Undo, ConfidenceLevel::High) => true,
        (VoiceAction::FreeForm { .. }, _) => true,
        _ => false, // Low confidence stop/undo needs confirmation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approve_command() {
        let cmd = parse_command("hydra approve");
        assert_eq!(cmd.action, VoiceAction::Approve);
        assert_eq!(cmd.confidence, ConfidenceLevel::High);

        let cmd2 = parse_command("yes");
        assert_eq!(cmd2.action, VoiceAction::Approve);

        let cmd3 = parse_command("go ahead");
        assert_eq!(cmd3.action, VoiceAction::Approve);

        let cmd4 = parse_command("hey hydra approve");
        assert_eq!(cmd4.action, VoiceAction::Approve);
    }

    #[test]
    fn test_deny_command() {
        let cmd = parse_command("hydra deny");
        assert_eq!(cmd.action, VoiceAction::Deny);
        assert_eq!(cmd.confidence, ConfidenceLevel::High);

        let cmd2 = parse_command("no");
        assert_eq!(cmd2.action, VoiceAction::Deny);

        let cmd3 = parse_command("nope");
        assert_eq!(cmd3.action, VoiceAction::Deny);
        assert_eq!(cmd3.confidence, ConfidenceLevel::Medium);
    }

    #[test]
    fn test_stop_command() {
        let cmd = parse_command("hydra stop");
        assert_eq!(cmd.action, VoiceAction::Stop);
        assert_eq!(cmd.confidence, ConfidenceLevel::High);

        let cmd2 = parse_command("cancel");
        assert_eq!(cmd2.action, VoiceAction::Stop);

        let cmd3 = parse_command("emergency stop");
        assert_eq!(cmd3.action, VoiceAction::Stop);

        let cmd4 = parse_command("kill it");
        assert_eq!(cmd4.action, VoiceAction::Stop);
    }

    #[test]
    fn test_explain_command() {
        let cmd = parse_command("hydra explain");
        assert_eq!(cmd.action, VoiceAction::Explain);
        assert_eq!(cmd.confidence, ConfidenceLevel::High);

        let cmd2 = parse_command("what are you doing");
        assert_eq!(cmd2.action, VoiceAction::Explain);

        let cmd3 = parse_command("why");
        assert_eq!(cmd3.action, VoiceAction::Explain);
    }

    #[test]
    fn test_status_command() {
        let cmd = parse_command("hydra status");
        assert_eq!(cmd.action, VoiceAction::Status);

        let cmd2 = parse_command("progress");
        assert_eq!(cmd2.action, VoiceAction::Status);
    }

    #[test]
    fn test_undo_command() {
        let cmd = parse_command("undo that");
        assert_eq!(cmd.action, VoiceAction::Undo);
        assert_eq!(cmd.confidence, ConfidenceLevel::High);
    }

    #[test]
    fn test_free_form_intent() {
        let cmd = parse_command("create a python function to sort a list");
        assert!(matches!(cmd.action, VoiceAction::FreeForm { .. }));
        if let VoiceAction::FreeForm { intent } = &cmd.action {
            assert_eq!(intent, "create a python function to sort a list");
        }
    }

    #[test]
    fn test_voice_command_parsing_case_insensitive() {
        let cmd = parse_command("HYDRA APPROVE");
        assert_eq!(cmd.action, VoiceAction::Approve);

        let cmd2 = parse_command("Stop");
        assert_eq!(cmd2.action, VoiceAction::Stop);
    }

    #[test]
    fn test_voice_command_whitespace_handling() {
        let cmd = parse_command("  hydra   approve  ");
        assert_eq!(cmd.action, VoiceAction::Approve);
    }

    #[test]
    fn test_voice_pipeline_stt_to_action() {
        // Simulate: STT output → command parse → safety check → action
        let stt_output = "hydra approve";
        let command = parse_command(stt_output);
        assert_eq!(command.action, VoiceAction::Approve);
        assert!(is_safe_to_execute(&command));
    }

    #[test]
    fn test_low_confidence_stop_needs_confirmation() {
        let cmd = VoiceCommand {
            action: VoiceAction::Stop,
            raw_text: "maybe stop something".into(),
            confidence: ConfidenceLevel::Low,
        };
        assert!(
            !is_safe_to_execute(&cmd),
            "Low confidence stop should require confirmation"
        );
    }

    #[test]
    fn test_graceful_when_no_mic() {
        // Voice subsystem should init without error even with no audio hardware
        use crate::config::VoiceConfig;
        use crate::subsystem::VoiceSubsystem;

        let config = VoiceConfig {
            enabled: false,
            ..VoiceConfig::default()
        };
        let subsystem = VoiceSubsystem::init(config).unwrap();
        assert!(!subsystem.is_active());
    }

    #[test]
    fn test_graceful_when_no_speaker() {
        // TTS should fall back to text when no audio output
        use crate::tts::TtsEngine;

        let engine = TtsEngine::new(); // no model loaded
        let result = engine.synthesize_or_text("hello");
        assert!(result.is_err()); // Returns text fallback
        assert_eq!(result.unwrap_err(), "hello");
    }
}
