//! ReachEngine — the relentless connectivity coordinator.
//! Tries all paths. Records every outcome.
//! Cartography grows with every new system encountered.
//! FAILED does not exist for connectivity.

use crate::{
    errors::ReachError,
    resolver::PathResolver,
    session::{ReachSession, SessionState},
    target::ReachTarget,
};
use std::collections::HashMap;

/// Result of one reach attempt.
#[derive(Debug)]
pub struct ReachResult {
    pub target: String,
    pub connected: bool,
    pub paths_tried: usize,
    pub successful_path: Option<String>,
    pub hard_denied: bool,
    pub receipt_ids: Vec<String>,
}

/// The reach engine.
pub struct ReachEngine {
    resolver: PathResolver,
    sessions: HashMap<String, ReachSession>,
}

impl ReachEngine {
    pub fn new() -> Self {
        Self {
            resolver: PathResolver::new(),
            sessions: HashMap::new(),
        }
    }

    /// Attempt to reach a target. Tries all paths before giving up.
    pub fn reach(&mut self, address: impl Into<String>) -> Result<ReachResult, ReachError> {
        let address = address.into();
        let target = ReachTarget::new(&address);
        let _target_cls = target.class.label();
        let paths = self.resolver.resolve_paths(&target);

        let mut session = ReachSession::new(target);
        let mut paths_tried = 0;
        let mut successful_path = None;
        let mut receipt_ids = Vec::new();

        for path_type in paths {
            paths_tried += 1;
            let path_label = path_type.label();

            // Simulate connection attempt
            // In production: actually attempt via hydra-protocol
            let simulate_success = !address.contains("fail")
                && !address.contains("denied")
                && !address.contains("unreachable");

            let connected = session.attempt_path(path_type, simulate_success);

            // Collect receipts
            if let Some(last_path) = session.paths.last() {
                receipt_ids.push(last_path.receipt_id.clone());
            }

            if connected {
                successful_path = Some(path_label);
                break;
            }

            // Check for hard denial
            if matches!(session.state, SessionState::HardDenied { .. }) {
                let session_id = session.id.clone();
                self.sessions.insert(session_id, session);
                return Err(ReachError::HardDenied {
                    target: address,
                    reason: "Explicit credential rejection".into(),
                });
            }
        }

        let connected = session.is_connected();
        let hard_denied = matches!(session.state, SessionState::HardDenied { .. });
        let session_id = session.id.clone();
        self.sessions.insert(session_id, session);

        if !connected && !hard_denied {
            return Err(ReachError::NoPathFound {
                target: address,
                attempts: paths_tried,
            });
        }

        Ok(ReachResult {
            target: address,
            connected,
            paths_tried: paths_tried as usize,
            successful_path,
            hard_denied,
            receipt_ids,
        })
    }

    /// Close a session to a target.
    pub fn disconnect(&mut self, address: &str) {
        if let Some(session) = self
            .sessions
            .values_mut()
            .find(|s| s.target.address == address)
        {
            session.close();
        }
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.values().filter(|s| s.is_connected()).count()
    }

    pub fn total_session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "reach: sessions={} active={}",
            self.sessions.len(),
            self.active_session_count(),
        )
    }
}

impl Default for ReachEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_valid_target_succeeds() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("https://api.example.com/data");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.connected);
        assert!(result.paths_tried >= 1);
        assert!(result.successful_path.is_some());
        assert!(!result.receipt_ids.is_empty());
    }

    #[test]
    fn reach_github_repo() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("https://github.com/org/hydra-repo");
        assert!(result.is_ok());
        assert!(result.unwrap().connected);
    }

    #[test]
    fn reach_mainframe() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("mainframe.corp.internal/jcl/batch");
        assert!(result.is_ok());
        assert!(result.unwrap().connected);
    }

    #[test]
    fn unreachable_target_returns_error() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("https://unreachable.example.com/api");
        assert!(result.is_err());
        assert!(!result.unwrap_err().is_hard_stop());
    }

    #[test]
    fn explicit_denial_returns_hard_denied() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("https://denied.example.com/api");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.is_hard_stop());
        }
    }

    #[test]
    fn every_attempt_has_receipt() {
        let mut engine = ReachEngine::new();
        let result = engine.reach("https://api.example.com");
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.receipt_ids.is_empty());
        for id in &result.receipt_ids {
            assert!(!id.is_empty());
        }
    }

    #[test]
    fn summary_format() {
        let engine = ReachEngine::new();
        let s = engine.summary();
        assert!(s.contains("reach:"));
        assert!(s.contains("sessions="));
    }
}
