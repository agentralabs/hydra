use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Approval request sent to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub run_id: String,
    pub action: String,
    pub target: Option<String>,
    pub risk_score: f64,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// User response to an approval request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Denied { reason: String },
    Modified { new_action: String },
}

/// Status of an approval request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Modified,
    Expired,
    Cancelled,
}

/// Internal state for a pending approval
struct PendingApproval {
    request: ApprovalRequest,
    sender: oneshot::Sender<ApprovalDecision>,
}

/// Manages approval requests with timeout enforcement
pub struct ApprovalManager {
    pending: Arc<DashMap<String, PendingApproval>>,
    timeout: Duration,
    history: Arc<DashMap<String, (ApprovalRequest, ApprovalStatus)>>,
}

impl ApprovalManager {
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            timeout,
            history: Arc::new(DashMap::new()),
        }
    }

    pub fn with_default_timeout() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes
    }

    /// Create a new approval request and return a receiver for the decision
    pub fn request_approval(
        &self,
        run_id: &str,
        action: &str,
        target: Option<&str>,
        risk_score: f64,
        reason: &str,
    ) -> (ApprovalRequest, oneshot::Receiver<ApprovalDecision>) {
        let (tx, rx) = oneshot::channel();
        let now = Utc::now();
        let request = ApprovalRequest {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            action: action.into(),
            target: target.map(Into::into),
            risk_score,
            reason: reason.into(),
            created_at: now,
            expires_at: now
                + chrono::Duration::from_std(self.timeout)
                    .unwrap_or(chrono::Duration::seconds(300)),
        };

        self.pending.insert(
            request.id.clone(),
            PendingApproval {
                request: request.clone(),
                sender: tx,
            },
        );

        (request, rx)
    }

    /// Submit a decision for a pending approval
    pub fn submit_decision(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
    ) -> Result<(), ApprovalError> {
        let (_, pending) = self
            .pending
            .remove(approval_id)
            .ok_or(ApprovalError::NotFound)?;

        let status = match &decision {
            ApprovalDecision::Approved => ApprovalStatus::Approved,
            ApprovalDecision::Denied { .. } => ApprovalStatus::Denied,
            ApprovalDecision::Modified { .. } => ApprovalStatus::Modified,
        };

        self.history
            .insert(approval_id.into(), (pending.request, status));

        pending
            .sender
            .send(decision)
            .map_err(|_| ApprovalError::ReceiverDropped)
    }

    /// Wait for approval with timeout
    pub async fn wait_for_approval(
        &self,
        approval_id: &str,
        rx: oneshot::Receiver<ApprovalDecision>,
    ) -> Result<ApprovalDecision, ApprovalError> {
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(decision)) => Ok(decision),
            Ok(Err(_)) => {
                // Sender dropped (e.g., cancelled)
                self.expire_approval(approval_id);
                Err(ApprovalError::Cancelled)
            }
            Err(_) => {
                // Timeout
                self.expire_approval(approval_id);
                Err(ApprovalError::Timeout)
            }
        }
    }

    /// Expire a pending approval
    fn expire_approval(&self, approval_id: &str) {
        if let Some((_, pending)) = self.pending.remove(approval_id) {
            self.history.insert(
                approval_id.into(),
                (pending.request, ApprovalStatus::Expired),
            );
        }
    }

    /// Cancel all pending approvals (used by kill switch)
    pub fn cancel_all(&self) -> usize {
        let ids: Vec<String> = self.pending.iter().map(|e| e.key().clone()).collect();
        let count = ids.len();
        for id in ids {
            if let Some((_, pending)) = self.pending.remove(&id) {
                let _ = pending.sender.send(ApprovalDecision::Denied {
                    reason: "Kill switch activated".into(),
                });
                self.history
                    .insert(id, (pending.request, ApprovalStatus::Cancelled));
            }
        }
        count
    }

    /// Get count of pending approvals
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if an approval is pending
    pub fn is_pending(&self, approval_id: &str) -> bool {
        self.pending.contains_key(approval_id)
    }

    /// List all pending approval requests
    pub fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.pending
            .iter()
            .map(|e| e.value().request.clone())
            .collect()
    }

    /// Get approval history status
    pub fn get_status(&self, approval_id: &str) -> Option<ApprovalStatus> {
        self.history.get(approval_id).map(|e| e.value().1.clone())
    }
}

impl Default for ApprovalManager {
    fn default() -> Self {
        Self::with_default_timeout()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalError {
    NotFound,
    Timeout,
    Cancelled,
    ReceiverDropped,
}

impl std::fmt::Display for ApprovalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalError::NotFound => write!(f, "Approval request not found"),
            ApprovalError::Timeout => write!(f, "Approval request timed out"),
            ApprovalError::Cancelled => write!(f, "Approval request was cancelled"),
            ApprovalError::ReceiverDropped => write!(f, "Approval receiver was dropped"),
        }
    }
}

impl std::error::Error for ApprovalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_approval_manager() {
        let mgr = ApprovalManager::with_default_timeout();
        assert_eq!(mgr.pending_count(), 0);
        assert!(mgr.list_pending().is_empty());
    }

    #[test]
    fn test_request_approval_creates_pending() {
        let mgr = ApprovalManager::with_default_timeout();
        let (req, _rx) = mgr.request_approval("run-1", "deploy", Some("prod"), 0.8, "high risk");
        assert!(!req.id.is_empty());
        assert_eq!(req.run_id, "run-1");
        assert_eq!(req.action, "deploy");
        assert_eq!(req.target, Some("prod".to_string()));
        assert_eq!(req.risk_score, 0.8);
        assert_eq!(mgr.pending_count(), 1);
        assert!(mgr.is_pending(&req.id));
    }

    #[test]
    fn test_submit_approved_decision() {
        let mgr = ApprovalManager::with_default_timeout();
        let (req, _rx) = mgr.request_approval("run-1", "deploy", None, 0.5, "test");
        let id = req.id.clone();
        let result = mgr.submit_decision(&id, ApprovalDecision::Approved);
        assert!(result.is_ok());
        assert_eq!(mgr.pending_count(), 0);
        assert!(!mgr.is_pending(&id));
        assert_eq!(mgr.get_status(&id), Some(ApprovalStatus::Approved));
    }

    #[test]
    fn test_submit_denied_decision() {
        let mgr = ApprovalManager::with_default_timeout();
        let (req, _rx) = mgr.request_approval("run-1", "delete", None, 0.9, "dangerous");
        let id = req.id.clone();
        mgr.submit_decision(&id, ApprovalDecision::Denied { reason: "too risky".into() }).unwrap();
        assert_eq!(mgr.get_status(&id), Some(ApprovalStatus::Denied));
    }

    #[test]
    fn test_submit_modified_decision() {
        let mgr = ApprovalManager::with_default_timeout();
        let (req, _rx) = mgr.request_approval("run-1", "deploy", None, 0.5, "test");
        let id = req.id.clone();
        mgr.submit_decision(&id, ApprovalDecision::Modified { new_action: "deploy-staging".into() }).unwrap();
        assert_eq!(mgr.get_status(&id), Some(ApprovalStatus::Modified));
    }

    #[test]
    fn test_submit_not_found() {
        let mgr = ApprovalManager::with_default_timeout();
        let result = mgr.submit_decision("nonexistent", ApprovalDecision::Approved);
        assert_eq!(result, Err(ApprovalError::NotFound));
    }

    #[test]
    fn test_cancel_all() {
        let mgr = ApprovalManager::with_default_timeout();
        mgr.request_approval("r1", "a1", None, 0.1, "test");
        mgr.request_approval("r2", "a2", None, 0.2, "test");
        mgr.request_approval("r3", "a3", None, 0.3, "test");
        assert_eq!(mgr.pending_count(), 3);
        let cancelled = mgr.cancel_all();
        assert_eq!(cancelled, 3);
        assert_eq!(mgr.pending_count(), 0);
    }

    #[test]
    fn test_list_pending() {
        let mgr = ApprovalManager::with_default_timeout();
        mgr.request_approval("r1", "a1", None, 0.1, "test");
        mgr.request_approval("r2", "a2", None, 0.2, "test");
        let pending = mgr.list_pending();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_approval_request_has_expiry() {
        let mgr = ApprovalManager::new(Duration::from_secs(60));
        let (req, _rx) = mgr.request_approval("r1", "a1", None, 0.5, "test");
        assert!(req.expires_at > req.created_at);
    }

    #[test]
    fn test_approval_error_display() {
        assert_eq!(ApprovalError::NotFound.to_string(), "Approval request not found");
        assert_eq!(ApprovalError::Timeout.to_string(), "Approval request timed out");
        assert_eq!(ApprovalError::Cancelled.to_string(), "Approval request was cancelled");
        assert_eq!(ApprovalError::ReceiverDropped.to_string(), "Approval receiver was dropped");
    }

    #[test]
    fn test_default_manager() {
        let mgr = ApprovalManager::default();
        assert_eq!(mgr.pending_count(), 0);
    }

    #[test]
    fn test_approval_no_target() {
        let mgr = ApprovalManager::with_default_timeout();
        let (req, _rx) = mgr.request_approval("r1", "action", None, 0.0, "no target");
        assert!(req.target.is_none());
    }
}
