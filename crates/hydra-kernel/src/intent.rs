//! Intent parsing — converts raw text into kernel commands.
//!
//! This is the bridge between natural language and the active loop.
//! In v0.1.0, this is keyword-based. Future versions will use LLM
//! micro-classification via the sister network.

use crate::loop_active::ActiveCommand;
use hydra_animus::{ResolvedIntent, SignalId, text_to_signal};

/// Parse a raw text intent from the principal into an ActiveCommand.
/// Returns the command and the resolved intent signal.
pub fn parse_intent(
    raw_text: &str,
    source_tier: u8,
) -> Result<(ActiveCommand, ResolvedIntent), String> {
    let resolved = text_to_signal(raw_text, source_tier, SignalId::identity())
        .map_err(|e| format!("Intent resolution failed: {e}"))?;

    let trimmed = raw_text.trim().to_lowercase();

    let command = if trimmed == "shutdown" || trimmed == "quit" || trimmed == "exit" {
        ActiveCommand::Shutdown
    } else if trimmed == "status" || trimmed == "state" {
        ActiveCommand::QueryState
    } else if let Some(rest) = trimmed.strip_prefix("resume ") {
        ActiveCommand::ResumeTask {
            task_id: rest.trim().to_string(),
        }
    } else {
        ActiveCommand::Execute {
            description: raw_text.to_string(),
        }
    };

    Ok((command, resolved))
}

/// Convert a resolved intent into a signal for the Animus bus.
/// This wraps the intent's signal for bus routing.
pub fn intent_to_signal(resolved: &ResolvedIntent) -> hydra_animus::Signal {
    resolved.signal.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_execute_intent() {
        let (cmd, resolved) = parse_intent("build the project", 2).expect("should parse");
        assert!(matches!(cmd, ActiveCommand::Execute { .. }));
        assert!(!resolved.signal.is_orphan());
    }

    #[test]
    fn parse_shutdown_intent() {
        let (cmd, _) = parse_intent("shutdown", 2).expect("should parse");
        assert!(matches!(cmd, ActiveCommand::Shutdown));
    }

    #[test]
    fn parse_status_intent() {
        let (cmd, _) = parse_intent("status", 2).expect("should parse");
        assert!(matches!(cmd, ActiveCommand::QueryState));
    }

    #[test]
    fn parse_resume_intent() {
        let (cmd, _) = parse_intent("resume task-123", 2).expect("should parse");
        match cmd {
            ActiveCommand::ResumeTask { task_id } => {
                assert_eq!(task_id, "task-123");
            }
            _ => panic!("Expected ResumeTask"),
        }
    }

    #[test]
    fn intent_to_signal_produces_valid_signal() {
        let (_, resolved) = parse_intent("test command", 2).expect("should parse");
        let signal = intent_to_signal(&resolved);
        assert!(signal.chain_is_complete());
    }
}
