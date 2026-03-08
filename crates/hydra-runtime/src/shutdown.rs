use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::event_bus::EventBus;
use crate::kill_switch::KillSwitch;
use crate::sse::SseEvent;
use crate::task_registry::TaskRegistry;

/// Shutdown sequence — 9 steps
pub struct ShutdownSequence {
    shutdown_flag: Arc<AtomicBool>,
    graceful_timeout: Duration,
}

impl ShutdownSequence {
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            graceful_timeout: Duration::from_secs(30),
        }
    }

    /// Get the shutdown flag (for checking in other components)
    pub fn flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }

    /// Is shutdown in progress?
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Execute graceful shutdown with real task cancellation
    pub async fn execute_with_registry(
        &self,
        event_bus: &EventBus,
        reason: &str,
        kill_switch: Option<&KillSwitch>,
        task_registry: Option<&TaskRegistry>,
        checkpoint_path: Option<&Path>,
    ) -> ShutdownResult {
        let start = Instant::now();

        // 1. Stop accepting new runs
        self.shutdown_flag.store(true, Ordering::SeqCst);

        // 2. Activate kill switch (graceful)
        if let Some(ks) = kill_switch {
            ks.graceful_stop(reason);
        }

        // 3. Cancel active runs via task registry
        let cancelled_count = if let Some(registry) = task_registry {
            registry.cancel_all()
        } else {
            0
        };

        if cancelled_count > 0 {
            tracing::info!("Cancelled {} active runs during shutdown", cancelled_count);
        }

        // 4. Save checkpoint if path provided
        if let Some(path) = checkpoint_path {
            Self::save_shutdown_checkpoint(path);
        }

        // 5-7: Flush ledger, close sisters, release lock (done by caller)

        // 8. Emit shutdown event
        event_bus.publish(SseEvent::system_shutdown(reason));

        // 9. Determine exit code
        let duration = start.elapsed();
        let forced = duration > self.graceful_timeout;

        ShutdownResult {
            clean: !forced,
            duration_ms: duration.as_millis() as u64,
            exit_code: if forced { 1 } else { 0 },
            reason: reason.into(),
            cancelled_runs: cancelled_count,
        }
    }

    /// Execute graceful shutdown (backward compatible)
    pub async fn execute(&self, event_bus: &EventBus, reason: &str) -> ShutdownResult {
        self.execute_with_registry(event_bus, reason, None, None, None)
            .await
    }

    /// Save a shutdown checkpoint marker
    fn save_shutdown_checkpoint(path: &Path) {
        let checkpoint = serde_json::json!({
            "type": "graceful_shutdown",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Ok(data) = serde_json::to_string_pretty(&checkpoint) {
            let _ = std::fs::write(path, data);
            tracing::info!("Saved shutdown checkpoint to {}", path.display());
        }
    }

    /// Force shutdown (second SIGINT)
    pub fn force_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }
}

impl Default for ShutdownSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ShutdownResult {
    pub clean: bool,
    pub duration_ms: u64,
    pub exit_code: i32,
    pub reason: String,
    pub cancelled_runs: usize,
}
