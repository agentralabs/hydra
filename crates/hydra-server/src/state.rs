use std::sync::Arc;
use std::time::{Duration, Instant};

use hydra_db::HydraDb;
use hydra_ledger::ReceiptLedger;
use hydra_runtime::EventBus;

/// Shared server state
pub struct AppState {
    pub db: HydraDb,
    pub event_bus: Arc<EventBus>,
    pub ledger: ReceiptLedger,
    pub server_mode: bool,
    pub auth_token: Option<String>,
    started_at: Instant,
}

impl AppState {
    pub fn new(db: HydraDb, server_mode: bool, auth_token: Option<String>) -> Self {
        Self {
            db,
            event_bus: Arc::new(EventBus::new(1024)),
            ledger: ReceiptLedger::new(),
            server_mode,
            auth_token,
            started_at: Instant::now(),
        }
    }

    /// Create AppState from shared components (for spawned tasks)
    pub fn new_from_shared(
        db: HydraDb,
        event_bus: Arc<EventBus>,
        ledger: ReceiptLedger,
        server_mode: bool,
        auth_token: Option<String>,
    ) -> Self {
        Self {
            db,
            event_bus,
            ledger,
            server_mode,
            auth_token,
            started_at: Instant::now(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
