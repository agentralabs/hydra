use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ReachError {
    #[error("No path found to target '{target}' after {attempts} attempts")]
    NoPathFound { target: String, attempts: u32 },

    #[error("Hard denial: explicit credential rejection for '{target}': {reason}")]
    HardDenied { target: String, reason: String },

    #[error("Target '{target}' is rate limiting — retry after {retry_after_seconds}s")]
    RateLimited {
        target: String,
        retry_after_seconds: u64,
    },

    #[error("Session limit reached for target '{target}' ({max} max)")]
    SessionLimitReached { target: String, max: usize },

    #[error("Target '{target}' unreachable: {reason}")]
    Unreachable { target: String, reason: String },
}

impl ReachError {
    /// True if this is a hard stop (explicit denial, not a navigational obstacle).
    pub fn is_hard_stop(&self) -> bool {
        matches!(self, Self::HardDenied { .. })
    }
}
