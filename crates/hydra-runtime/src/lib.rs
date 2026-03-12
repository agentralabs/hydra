pub mod approval;
pub mod hardening;
pub mod boot;
pub mod cognitive;
pub mod completion;
pub mod config;
pub mod daemon;
pub mod degradation;
pub mod event_bus;
pub mod filesystem;
pub mod jsonrpc;
pub mod kill_switch;
pub mod lock;
pub mod notifications;
pub mod offline;
pub mod private_features;
pub mod proactive;
pub mod profile;
pub mod runtime;
pub mod shutdown;
pub mod sse;
pub mod task_registry;
pub mod tasks;
pub mod undo;

pub use approval::{
    ApprovalDecision, ApprovalError, ApprovalManager, ApprovalRequest, ApprovalStatus,
};
pub use boot::BootSequence;
pub use cognitive::{CognitiveLoopConfig, LlmPhaseHandler};
pub use config::{HydraRuntimeConfig, LimitsConfig, LlmConfigSection, ResourceProfile};
pub use daemon::{
    ConsolidationDaemon, DaemonConfig, DaemonTask, OpportunisticRunner, ScheduledTask, TaskId,
    TaskResult, TaskScheduler, TaskStatus,
};
pub use degradation::{
    DegradationAction, DegradationLevel, DegradationManager, DegradationPolicy, PolicyConfig,
    ResourceMonitor, ResourceSnapshot,
};
pub use event_bus::EventBus;
pub use filesystem::init_filesystem;
pub use jsonrpc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, RpcErrorCodes};
pub use kill_switch::{KillSignal, KillSwitch};
pub use lock::InstanceLock;
pub use notifications::{Notification, NotificationAction, NotificationManager, NotificationUrgency};

pub use offline::{
    ConflictStrategy, ConnectivityMonitor, ConnectivityState, OfflineConfig, OfflineManager,
    PendingAction, PendingSyncQueue, SyncEngine, SyncPriority, SyncResult,
};
pub use private_features::PrivateFeatures;
pub use runtime::HydraRuntime;
pub use shutdown::ShutdownSequence;
pub use sse::SseEvent;
pub use profile::{InterfaceMode, Theme, UserPreferences, UserProfile};
pub use task_registry::TaskRegistry;
pub use tasks::{HydraTaskStatus, Task, TaskManager};
pub use undo::{GenericAction, UndoError, UndoStack, UndoableAction};

pub use completion::{ChangeType, ChangeSummary, CompletionStats, CompletionSummary};
pub use proactive::{AlertSeverity, DecisionOption, EventType, ProactiveEngine, ProactiveUpdate};
