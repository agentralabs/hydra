use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;
use uuid::Uuid;

use hydra_core::types::{DecisionOption, DecisionRequest, DecisionResponse};

/// Result of a decision request
#[derive(Debug, Clone)]
pub enum DecisionResult {
    /// User chose an option
    Chosen { option_index: usize, label: String },
    /// Timed out with a default
    TimedOutWithDefault { option_index: usize, label: String },
    /// Timed out with no default — aborted
    TimedOutAborted,
    /// User disconnected
    Disconnected,
}

impl DecisionResult {
    pub fn timed_out(&self) -> bool {
        matches!(
            self,
            Self::TimedOutWithDefault { .. } | Self::TimedOutAborted
        )
    }

    pub fn aborted(&self) -> bool {
        matches!(self, Self::TimedOutAborted | Self::Disconnected)
    }

    pub fn chosen_index(&self) -> Option<usize> {
        match self {
            Self::Chosen { option_index, .. } | Self::TimedOutWithDefault { option_index, .. } => {
                Some(*option_index)
            }
            _ => None,
        }
    }
}

/// Pending approval that survives system sleep (EC-UX-010)
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub request: DecisionRequest,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Engine for decision requests and approvals
pub struct DecisionEngine {
    pending_approvals: Arc<Mutex<Vec<PendingApproval>>>,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self {
            pending_approvals: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Request a decision from the user with timeout handling
    /// EC-UX-006: If timeout expires and no default, returns TimedOutAborted
    pub async fn request_decision(
        &self,
        request: DecisionRequest,
        response_rx: Option<oneshot::Receiver<DecisionResponse>>,
    ) -> DecisionResult {
        let timeout = request
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        // Store as pending approval (survives sleep — EC-UX-010)
        self.pending_approvals.lock().push(PendingApproval {
            request: request.clone(),
            created_at: chrono::Utc::now(),
        });

        match response_rx {
            Some(rx) => match tokio::time::timeout(timeout, rx).await {
                Ok(Ok(response)) => {
                    self.remove_pending(&request.id);
                    let idx = response.chosen_option;
                    let label = request
                        .options
                        .get(idx)
                        .map(|o| o.label.clone())
                        .unwrap_or_default();
                    DecisionResult::Chosen {
                        option_index: idx,
                        label,
                    }
                }
                Ok(Err(_)) => {
                    // Channel closed — user disconnected
                    DecisionResult::Disconnected
                }
                Err(_) => {
                    // Timeout
                    self.handle_timeout(&request)
                }
            },
            None => {
                // No response channel — simulate timeout
                tokio::time::sleep(timeout).await;
                self.handle_timeout(&request)
            }
        }
    }

    fn handle_timeout(&self, request: &DecisionRequest) -> DecisionResult {
        // EC-UX-006: No default → abort, don't pick random
        match request.default {
            Some(idx) => {
                self.remove_pending(&request.id);
                let label = request
                    .options
                    .get(idx)
                    .map(|o| o.label.clone())
                    .unwrap_or_default();
                DecisionResult::TimedOutWithDefault {
                    option_index: idx,
                    label,
                }
            }
            None => {
                // Don't remove from pending — keep for re-presentation (EC-UX-010)
                DecisionResult::TimedOutAborted
            }
        }
    }

    /// Check if there are pending approvals (EC-UX-010: survives sleep/wake)
    pub fn has_pending_approval(&self) -> bool {
        !self.pending_approvals.lock().is_empty()
    }

    /// Get all pending approvals
    pub fn pending_approvals(&self) -> Vec<PendingApproval> {
        self.pending_approvals.lock().clone()
    }

    /// Remove a pending approval by request ID
    fn remove_pending(&self, request_id: &Uuid) {
        self.pending_approvals
            .lock()
            .retain(|p| p.request.id != *request_id);
    }

    /// Clear all pending approvals
    pub fn clear_pending(&self) {
        self.pending_approvals.lock().clear();
    }

    /// Maximum number of options per decision (cognitively manageable)
    pub const MAX_OPTIONS: usize = 4;

    /// Build a decision request with sensible defaults.
    /// Enforces maximum 4 options — extra options are truncated.
    pub fn build_request(
        question: impl Into<String>,
        options: Vec<DecisionOption>,
        timeout_seconds: u64,
        default: Option<usize>,
    ) -> DecisionRequest {
        let mut options = options;
        options.truncate(Self::MAX_OPTIONS);
        // If default points beyond the truncated list, clear it
        let default = default.filter(|&d| d < options.len());
        DecisionRequest {
            id: Uuid::new_v4(),
            question: question.into(),
            options,
            timeout_seconds: Some(timeout_seconds),
            default,
        }
    }

    /// Validate that a request doesn't exceed option limits
    pub fn validate_request(request: &DecisionRequest) -> bool {
        request.options.len() <= Self::MAX_OPTIONS
            && request.default.map_or(true, |d| d < request.options.len())
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}
