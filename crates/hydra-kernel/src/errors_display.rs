//! User-facing error messages — maps internal errors to actionable guidance.
//! Every error tells the user what happened AND what to do about it.

use crate::loop_::llm::LlmError;

/// Convert an LLM error into a human-friendly message with actionable fix.
pub fn humanize_llm_error(error: &LlmError) -> String {
    match error {
        LlmError::MissingKey { provider, key_env } => {
            format!(
                "No API key for {provider}.\n\
                 Fix: Set the {key_env} environment variable.\n\
                 Example: export {key_env}=sk-your-key-here"
            )
        }
        LlmError::RateLimited { provider } => {
            format!(
                "{provider} rate limit hit — too many requests.\n\
                 Hydra will retry automatically with backoff.\n\
                 If persistent: check your plan limits at the provider dashboard."
            )
        }
        LlmError::Network { message } => {
            let clean_msg = message.chars().take(100).collect::<String>();
            format!(
                "Network issue: {clean_msg}\n\
                 Check: Is your internet connection working?\n\
                 Check: Is {msg} reachable?",
                msg = if message.contains("ollama") {
                    "Ollama (localhost:11434)"
                } else {
                    "the API endpoint"
                }
            )
        }
        LlmError::ProviderError { provider, message } => {
            let clean_msg = message.chars().take(120).collect::<String>();
            if message.contains("401") || message.contains("Unauthorized") {
                format!(
                    "{provider}: Invalid API key.\n\
                     Fix: Check your API key is correct and active.\n\
                     Run: hydra --check-key"
                )
            } else if message.contains("404") || message.contains("Not Found") {
                format!(
                    "{provider}: Model not found.\n\
                     Fix: Check HYDRA_LLM_MODEL is set to a valid model name.\n\
                     Common models: claude-sonnet-4-20250514, gpt-4o, gemini-pro"
                )
            } else if message.contains("500") || message.contains("Internal Server") {
                format!(
                    "{provider}: Server error (their side, not yours).\n\
                     Hydra will retry. If persistent, check {provider} status page."
                )
            } else {
                format!(
                    "{provider} error: {clean_msg}\n\
                     Check the provider's API documentation for details."
                )
            }
        }
    }
}

/// Humanize a boot/startup error.
pub fn humanize_boot_error(phase: &str, error: &str) -> String {
    match phase {
        "ConstitutionVerify" => format!(
            "Constitutional verification failed: {error}\n\
             This is a critical safety check. Hydra cannot start without it.\n\
             Fix: Ensure hydra-constitution crate is intact."
        ),
        "MemoryResume" => format!(
            "Memory system failed to start: {error}\n\
             Fix: Check ~/.hydra/data/hydra.amem exists and is readable.\n\
             If corrupted: rename it and Hydra will create a fresh one."
        ),
        "GenomeLoad" => format!(
            "Genome database issue: {error}\n\
             Fix: Check ~/.hydra/data/genome.db is not locked by another process.\n\
             If corrupted: delete it and skills will repopulate it on next boot."
        ),
        _ => format!(
            "Boot phase '{phase}' failed: {error}\n\
             Check ~/.hydra/data/ for corrupted files.\n\
             Run: hydra --self-repair"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_message_actionable() {
        let err = LlmError::MissingKey {
            provider: "Anthropic".into(),
            key_env: "ANTHROPIC_API_KEY".into(),
        };
        let msg = humanize_llm_error(&err);
        assert!(msg.contains("ANTHROPIC_API_KEY"));
        assert!(msg.contains("export"));
    }

    #[test]
    fn rate_limit_message_calm() {
        let err = LlmError::RateLimited {
            provider: "OpenAI".into(),
        };
        let msg = humanize_llm_error(&err);
        assert!(msg.contains("retry"));
        assert!(!msg.contains("error")); // Should not be alarming
    }

    #[test]
    fn network_error_has_check_steps() {
        let err = LlmError::Network {
            message: "connection refused to ollama".into(),
        };
        let msg = humanize_llm_error(&err);
        assert!(msg.contains("internet"));
        assert!(msg.contains("Ollama"));
    }

    #[test]
    fn provider_401_suggests_key_check() {
        let err = LlmError::ProviderError {
            provider: "Anthropic".into(),
            message: "401 Unauthorized".into(),
        };
        let msg = humanize_llm_error(&err);
        assert!(msg.contains("Invalid API key"));
    }

    #[test]
    fn boot_error_memory_has_fix() {
        let msg = humanize_boot_error("MemoryResume", "file not found");
        assert!(msg.contains(".hydra/data"));
        assert!(msg.contains("Fix"));
    }
}
