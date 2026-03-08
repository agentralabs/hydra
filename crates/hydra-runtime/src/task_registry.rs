use std::sync::Arc;

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Tracks active runs for cancellation and lifecycle management
pub struct TaskRegistry {
    handles: Arc<DashMap<String, JoinHandle<()>>>,
    tokens: Arc<DashMap<String, CancellationToken>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(DashMap::new()),
            tokens: Arc::new(DashMap::new()),
        }
    }

    /// Register a new run with its handle and cancellation token
    pub fn register(&self, run_id: &str, handle: JoinHandle<()>, token: CancellationToken) {
        self.handles.insert(run_id.into(), handle);
        self.tokens.insert(run_id.into(), token);
    }

    /// Create a cancellation token for a run (register handle separately)
    pub fn create_token(&self, run_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        self.tokens.insert(run_id.into(), token.clone());
        token
    }

    /// Cancel a specific run
    pub fn cancel(&self, run_id: &str) -> bool {
        let cancelled = if let Some(token) = self.tokens.get(run_id) {
            token.cancel();
            true
        } else {
            false
        };

        if let Some((_, handle)) = self.handles.remove(run_id) {
            handle.abort();
        }
        self.tokens.remove(run_id);

        cancelled
    }

    /// Cancel all active runs (used by kill switch)
    pub fn cancel_all(&self) -> usize {
        let ids: Vec<String> = self.tokens.iter().map(|e| e.key().clone()).collect();
        let count = ids.len();
        for id in &ids {
            if let Some(token) = self.tokens.get(id) {
                token.cancel();
            }
        }
        for id in &ids {
            if let Some((_, handle)) = self.handles.remove(id) {
                handle.abort();
            }
            self.tokens.remove(id);
        }
        count
    }

    /// Get count of active runs
    pub fn active_count(&self) -> usize {
        self.tokens.len()
    }

    /// Check if a run is active
    pub fn is_active(&self, run_id: &str) -> bool {
        self.tokens.contains_key(run_id)
    }

    /// Check if a run is cancelled
    pub fn is_cancelled(&self, run_id: &str) -> bool {
        self.tokens
            .get(run_id)
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    /// Remove completed run from tracking
    pub fn remove(&self, run_id: &str) {
        self.handles.remove(run_id);
        self.tokens.remove(run_id);
    }

    /// List active run IDs
    pub fn list_active(&self) -> Vec<String> {
        self.tokens.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}
