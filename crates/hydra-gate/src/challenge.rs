use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Word lists for generating challenge phrases
const WORDS: &[&str] = &[
    "ALPHA", "BETA", "DELTA", "GAMMA", "CONFIRM", "ECHO", "FOXTROT",
];

const NUMBERS: &[&str] = &["ONE", "TWO", "THREE", "FIVE", "SEVEN", "NINE"];

/// A generated challenge phrase that must be repeated to authorize an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengePhrase {
    /// The phrase the user must type
    pub phrase: String,
    /// The action this challenge authorizes
    pub action_id: String,
    /// When this challenge expires
    pub expires_at: DateTime<Utc>,
}

impl ChallengePhrase {
    /// Whether this challenge has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Manages active challenge phrases for high-risk action confirmation
pub struct ChallengeManager {
    active_challenges: HashMap<String, ChallengePhrase>,
    /// How long challenges remain valid (default: 120 seconds)
    ttl_seconds: i64,
}

impl Default for ChallengeManager {
    fn default() -> Self {
        Self::new(120)
    }
}

impl ChallengeManager {
    /// Create a new manager with the given TTL in seconds
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            active_challenges: HashMap::new(),
            ttl_seconds,
        }
    }

    /// Generate a challenge phrase for the given action.
    /// Replaces any existing challenge for the same action_id.
    pub fn generate(&mut self, action_id: impl Into<String>) -> ChallengePhrase {
        let action_id = action_id.into();

        let word = pick(WORDS);
        let number = pick(NUMBERS);
        let phrase = format!("{} {}", word, number);

        let challenge = ChallengePhrase {
            phrase,
            action_id: action_id.clone(),
            expires_at: Utc::now() + Duration::seconds(self.ttl_seconds),
        };

        self.active_challenges
            .insert(action_id, challenge.clone());
        challenge
    }

    /// Validate user input against the active challenge for the given action.
    /// Comparison is case-insensitive.
    /// A valid challenge is consumed (one-time use).
    pub fn validate(&mut self, action_id: &str, input: &str) -> bool {
        let valid = match self.active_challenges.get(action_id) {
            Some(challenge) if !challenge.is_expired() => {
                challenge.phrase.eq_ignore_ascii_case(input.trim())
            }
            _ => false,
        };

        if valid {
            self.active_challenges.remove(action_id);
        }

        valid
    }

    /// Remove all expired challenges
    pub fn expire_old(&mut self) {
        let now = Utc::now();
        self.active_challenges
            .retain(|_, challenge| challenge.expires_at > now);
    }

    /// Number of active (non-expired) challenges
    pub fn active_count(&self) -> usize {
        self.active_challenges
            .values()
            .filter(|c| !c.is_expired())
            .count()
    }
}

/// Simple deterministic-ish picker using a fast hash of the current timestamp.
/// Not cryptographically secure — this is a UX confirmation, not a secret.
fn pick<'a>(list: &'a [&'a str]) -> &'a str {
    let nanos = Utc::now().timestamp_subsec_nanos() as usize;
    list[nanos % list.len()]
}
