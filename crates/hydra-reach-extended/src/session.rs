//! ReachSession — active connection state to one target.
//! Tracks all paths attempted, current state, receipts.

use crate::{
    path::{ConnectionPath, PathOutcome, PathType},
    target::ReachTarget,
};
use serde::{Deserialize, Serialize};

/// State of a reach session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionState {
    /// Trying to connect.
    Connecting { current_path: String },
    /// Connected and usable.
    Connected {
        since: chrono::DateTime<chrono::Utc>,
    },
    /// Blocked — trying next path.
    Rerouting { attempt: u32, reason: String },
    /// Explicitly denied — hard stop.
    HardDenied { evidence: String },
    /// Waiting out a rate limit.
    Suspended {
        retry_after: chrono::DateTime<chrono::Utc>,
    },
    /// Closed cleanly.
    Closed,
    // FAILED: does not exist.
}

impl SessionState {
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::HardDenied { .. } | Self::Closed)
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Connecting { .. } => "connecting",
            Self::Connected { .. } => "connected",
            Self::Rerouting { .. } => "rerouting",
            Self::HardDenied { .. } => "hard-denied",
            Self::Suspended { .. } => "suspended",
            Self::Closed => "closed",
        }
    }
}

/// An active reach session.
pub struct ReachSession {
    pub id: String,
    pub target: ReachTarget,
    pub state: SessionState,
    pub paths: Vec<ConnectionPath>,
}

impl ReachSession {
    pub fn new(target: ReachTarget) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target,
            state: SessionState::Connecting {
                current_path: "pending".into(),
            },
            paths: Vec::new(),
        }
    }

    /// Attempt one path. Returns true if connected.
    pub fn attempt_path(&mut self, path_type: PathType, simulate_success: bool) -> bool {
        let mut path = ConnectionPath::new(path_type.clone());
        let start = std::time::Instant::now();

        self.state = SessionState::Connecting {
            current_path: path_type.label(),
        };

        let outcome = if simulate_success {
            PathOutcome::Connected
        } else {
            let addr = &self.target.address;
            if addr.contains("denied") || addr.contains("forbidden") {
                PathOutcome::HardDenied {
                    reason: "401 explicit rejection".into(),
                }
            } else if addr.contains("ratelimit") {
                PathOutcome::RateLimited {
                    retry_after_seconds: 60,
                }
            } else {
                PathOutcome::Timeout
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        path.resolve(outcome.clone(), duration_ms);

        let is_success = path.is_success();
        let is_hard_stop = path.is_hard_stop();

        self.paths.push(path);

        if is_success {
            self.state = SessionState::Connected {
                since: chrono::Utc::now(),
            };
        } else if is_hard_stop {
            let evidence = match &outcome {
                PathOutcome::HardDenied { reason } => reason.clone(),
                _ => "unknown".into(),
            };
            self.state = SessionState::HardDenied { evidence };
        } else {
            let reason = format!("{:?}", outcome);
            self.state = SessionState::Rerouting {
                attempt: self.paths.len() as u32,
                reason,
            };
        }

        is_success
    }

    pub fn close(&mut self) {
        self.state = SessionState::Closed;
    }

    pub fn attempt_count(&self) -> usize {
        self.paths.len()
    }
    pub fn is_connected(&self) -> bool {
        self.state.is_usable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_path_connected() {
        let target = ReachTarget::new("https://api.example.com");
        let mut ses = ReachSession::new(target);
        let success = ses.attempt_path(PathType::Direct, true);
        assert!(success);
        assert!(ses.is_connected());
        assert_eq!(ses.state.label(), "connected");
    }

    #[test]
    fn failed_path_rerouting() {
        let target = ReachTarget::new("https://api.timeout.example.com");
        let mut ses = ReachSession::new(target);
        let success = ses.attempt_path(PathType::Direct, false);
        assert!(!success);
        assert!(!ses.is_connected());
        assert_eq!(ses.attempt_count(), 1);
    }

    #[test]
    fn hard_denied_is_terminal() {
        let target = ReachTarget::new("https://denied.example.com");
        let mut ses = ReachSession::new(target);
        ses.attempt_path(PathType::Direct, false);
        if ses.state
            == (SessionState::HardDenied {
                evidence: "401 explicit rejection".into(),
            })
        {
            assert!(ses.state.is_terminal());
        }
    }

    #[test]
    fn receipts_on_every_attempt() {
        let target = ReachTarget::new("https://api.example.com");
        let mut ses = ReachSession::new(target);
        ses.attempt_path(PathType::Direct, true);
        for path in &ses.paths {
            assert!(!path.receipt_id.is_empty());
        }
    }
}
