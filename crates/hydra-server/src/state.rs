use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hydra_db::{HydraDb, MessageStore};
use hydra_runtime::approval::ApprovalManager;
use hydra_runtime::degradation::DegradationManager;
use hydra_runtime::kill_switch::KillSwitch;
use hydra_runtime::{EventBus, TaskManager};
use parking_lot::Mutex;

use hydra_ledger::ReceiptLedger;
use hydra_native::{AgentSpawner, DecideEngine, InventionEngine, ProactiveNotifier};

/// Shared server state
pub struct AppState {
    pub db: HydraDb,
    pub event_bus: Arc<EventBus>,
    pub ledger: ReceiptLedger,
    pub message_store: MessageStore,
    pub task_manager: Arc<Mutex<TaskManager>>,
    pub approval_manager: ApprovalManager,
    pub kill_switch: KillSwitch,
    pub degradation_manager: DegradationManager,
    pub decide_engine: Arc<DecideEngine>,
    pub invention_engine: Arc<InventionEngine>,
    pub proactive_notifier: Arc<Mutex<ProactiveNotifier>>,
    pub agent_spawner: Arc<AgentSpawner>,
    pub profile_path: PathBuf,
    pub server_mode: bool,
    pub auth_token: Option<String>,
    started_at: Instant,
}

impl AppState {
    pub fn new(db: HydraDb, server_mode: bool, auth_token: Option<String>) -> Self {
        let message_store =
            MessageStore::new(db.connection()).expect("failed to init MessageStore");
        Self {
            db,
            event_bus: Arc::new(EventBus::new(1024)),
            ledger: ReceiptLedger::new(),
            message_store,
            task_manager: Arc::new(Mutex::new(TaskManager::new())),
            approval_manager: ApprovalManager::with_default_timeout(),
            kill_switch: KillSwitch::new(),
            degradation_manager: DegradationManager::with_defaults(),
            decide_engine: Arc::new(DecideEngine::new()),
            invention_engine: Arc::new(InventionEngine::new()),
            proactive_notifier: Arc::new(Mutex::new(ProactiveNotifier::new())),
            agent_spawner: Arc::new(AgentSpawner::new(100)),
            profile_path: hydra_runtime::profile::ProfileStorage::default_path(),
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
        let message_store =
            MessageStore::new(db.connection()).expect("failed to init MessageStore");
        Self {
            db,
            event_bus,
            ledger,
            message_store,
            task_manager: Arc::new(Mutex::new(TaskManager::new())),
            approval_manager: ApprovalManager::with_default_timeout(),
            kill_switch: KillSwitch::new(),
            degradation_manager: DegradationManager::with_defaults(),
            decide_engine: Arc::new(DecideEngine::new()),
            invention_engine: Arc::new(InventionEngine::new()),
            proactive_notifier: Arc::new(Mutex::new(ProactiveNotifier::new())),
            agent_spawner: Arc::new(AgentSpawner::new(100)),
            profile_path: hydra_runtime::profile::ProfileStorage::default_path(),
            server_mode,
            auth_token,
            started_at: Instant::now(),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}
