//! Challenge Phrase Gate — irreversible HIGH+ risk actions require
//! the user to type back a deterministic NATO-phonetic phrase.

use std::time::Instant;

/// Generate a deterministic challenge phrase from an action summary.
/// Returns a two-word NATO-phonetic phrase the user must type to confirm.
pub fn generate_challenge_phrase(action_summary: &str) -> String {
    let words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];
    let hash = action_summary
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let w1 = words[(hash % 6) as usize];
    let w2 = words[((hash >> 3) % 6) as usize];
    format!("{}-{}", w1, w2)
}

/// Challenge phrase gate for irreversible HIGH+ risk actions.
/// Requires the user to type back a generated phrase within a time window.
#[derive(Debug, Clone)]
pub struct ChallengePhraseGate {
    pub phrase: String,
    pub action_summary: String,
    pub issued_at: Instant,
    pub expires_in_secs: u64,
}

impl ChallengePhraseGate {
    /// Create a new challenge with a 30-second expiry window.
    pub fn new(action_summary: &str) -> Self {
        Self {
            phrase: generate_challenge_phrase(action_summary),
            action_summary: action_summary.to_string(),
            issued_at: Instant::now(),
            expires_in_secs: 30,
        }
    }

    /// Check if the challenge has expired.
    pub fn is_expired(&self) -> bool {
        self.issued_at.elapsed().as_secs() > self.expires_in_secs
    }

    /// Verify the user's input against the challenge phrase.
    /// Returns false if expired or if the phrase doesn't match.
    pub fn verify(&self, input: &str) -> bool {
        !self.is_expired() && input.trim().to_lowercase() == self.phrase
    }

    /// Whether this action warrants a challenge phrase.
    /// Only irreversible actions at HIGH or CRITICAL risk trigger this.
    pub fn should_challenge(risk_level: &str, command: &str) -> bool {
        let is_high_plus = matches!(risk_level, "high" | "critical");
        let is_irreversible = Self::is_irreversible_command(command);
        is_high_plus && is_irreversible
    }

    /// Heuristic: does this command perform an irreversible action?
    fn is_irreversible_command(command: &str) -> bool {
        let lower = command.to_lowercase();
        lower.contains("rm ") || lower.contains("delete")
            || lower.contains("drop ") || lower.contains("truncate")
            || lower.contains("format") || lower.contains("reset --hard")
            || lower.contains("force push") || lower.contains("push --force")
            || lower.contains("push -f") || lower.contains("--no-preserve-root")
    }
}
