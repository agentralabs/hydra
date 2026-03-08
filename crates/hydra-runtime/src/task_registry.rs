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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry_empty() {
        let reg = TaskRegistry::new();
        assert_eq!(reg.active_count(), 0);
        assert!(reg.list_active().is_empty());
    }

    #[test]
    fn test_create_token() {
        let reg = TaskRegistry::new();
        let _token = reg.create_token("run-1");
        assert!(reg.is_active("run-1"));
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn test_cancel_nonexistent() {
        let reg = TaskRegistry::new();
        assert!(!reg.cancel("nonexistent"));
    }

    #[test]
    fn test_cancel_existing() {
        let reg = TaskRegistry::new();
        reg.create_token("run-1");
        assert!(reg.cancel("run-1"));
        assert!(!reg.is_active("run-1"));
        assert_eq!(reg.active_count(), 0);
    }

    #[test]
    fn test_cancel_all() {
        let reg = TaskRegistry::new();
        reg.create_token("run-1");
        reg.create_token("run-2");
        reg.create_token("run-3");
        assert_eq!(reg.active_count(), 3);
        let count = reg.cancel_all();
        assert_eq!(count, 3);
        assert_eq!(reg.active_count(), 0);
    }

    #[test]
    fn test_is_cancelled() {
        let reg = TaskRegistry::new();
        let token = reg.create_token("run-1");
        assert!(!reg.is_cancelled("run-1"));
        token.cancel();
        assert!(reg.is_cancelled("run-1"));
    }

    #[test]
    fn test_is_cancelled_nonexistent() {
        let reg = TaskRegistry::new();
        assert!(!reg.is_cancelled("nonexistent"));
    }

    #[test]
    fn test_remove() {
        let reg = TaskRegistry::new();
        reg.create_token("run-1");
        reg.remove("run-1");
        assert!(!reg.is_active("run-1"));
    }

    #[test]
    fn test_list_active() {
        let reg = TaskRegistry::new();
        reg.create_token("run-a");
        reg.create_token("run-b");
        let active = reg.list_active();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"run-a".to_string()));
        assert!(active.contains(&"run-b".to_string()));
    }

    #[test]
    fn test_default() {
        let reg = TaskRegistry::default();
        assert_eq!(reg.active_count(), 0);
    }
}
