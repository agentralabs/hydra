//! ConnectionPath — one attempted route to a target.
//! Each path attempt is receipted. Failures recorded in antifragile.

use serde::{Deserialize, Serialize};

/// The type of path attempted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathType {
    /// Direct connection via detected protocol.
    Direct,
    /// Alternative tooling (different client library).
    AlternativeTooling { tool: String },
    /// Environment adaptation (proxy, VPN, etc.).
    EnvironmentAdapt { adaptation: String },
    /// Protocol switch (try a different protocol).
    ProtocolSwitch { from: String, to: String },
    /// Decompose request into smaller pieces.
    Decomposition { chunk_size: usize },
    /// Use a relay as intermediary.
    Relay { relay_address: String },
    /// Wait and retry (rate limit, temporary outage).
    Patience { wait_seconds: u64, reason: String },
    /// Delegate to a specialist fleet agent.
    AgentDelegation { agent_type: String },
}

impl PathType {
    pub fn label(&self) -> String {
        match self {
            Self::Direct => "direct".into(),
            Self::AlternativeTooling { tool } => format!("alt:{}", tool),
            Self::EnvironmentAdapt { adaptation } => format!("adapt:{}", adaptation),
            Self::ProtocolSwitch { to, .. } => format!("switch:{}", to),
            Self::Decomposition { chunk_size } => format!("decompose:{}", chunk_size),
            Self::Relay { .. } => "relay".into(),
            Self::Patience { wait_seconds, .. } => format!("patience:{}s", wait_seconds),
            Self::AgentDelegation { agent_type } => format!("agent:{}", agent_type),
        }
    }
}

/// The result of one path attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PathOutcome {
    /// Connection established and usable.
    Connected,
    /// Explicitly rejected — credentials invalid.
    HardDenied { reason: String },
    /// Rate limited — not a hard stop.
    RateLimited { retry_after_seconds: u64 },
    /// Timeout — try next path.
    Timeout,
    /// Network error — try next path.
    NetworkError { detail: String },
    /// Protocol mismatch — try different protocol.
    ProtocolMismatch { expected: String, got: String },
}

impl PathOutcome {
    pub fn is_hard_stop(&self) -> bool {
        matches!(self, Self::HardDenied { .. })
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Connected)
    }

    pub fn is_navigational(&self) -> bool {
        !self.is_hard_stop() && !self.is_success()
    }
}

/// One attempted connection path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPath {
    pub id: String,
    pub path_type: PathType,
    pub outcome: Option<PathOutcome>,
    pub receipt_id: String,
    pub attempted_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
}

impl ConnectionPath {
    pub fn new(path_type: PathType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            receipt_id: uuid::Uuid::new_v4().to_string(),
            path_type,
            outcome: None,
            attempted_at: chrono::Utc::now(),
            duration_ms: 0,
        }
    }

    pub fn resolve(&mut self, outcome: PathOutcome, duration_ms: u64) {
        self.outcome = Some(outcome);
        self.duration_ms = duration_ms;
    }

    pub fn is_success(&self) -> bool {
        self.outcome
            .as_ref()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    pub fn is_hard_stop(&self) -> bool {
        self.outcome
            .as_ref()
            .map(|o| o.is_hard_stop())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_starts_with_receipt() {
        let p = ConnectionPath::new(PathType::Direct);
        assert!(!p.receipt_id.is_empty());
        assert!(p.outcome.is_none());
    }

    #[test]
    fn hard_denied_is_hard_stop() {
        let o = PathOutcome::HardDenied {
            reason: "401".into(),
        };
        assert!(o.is_hard_stop());
        assert!(!o.is_navigational());
    }

    #[test]
    fn timeout_is_navigational() {
        let o = PathOutcome::Timeout;
        assert!(o.is_navigational());
        assert!(!o.is_hard_stop());
    }

    #[test]
    fn rate_limited_is_navigational() {
        let o = PathOutcome::RateLimited {
            retry_after_seconds: 60,
        };
        assert!(o.is_navigational());
        assert!(!o.is_hard_stop());
    }
}
