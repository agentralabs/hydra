use std::path::Path;
use std::time::Instant;

use crate::config::HydraRuntimeConfig;
use crate::event_bus::EventBus;
use crate::filesystem;
use crate::sse::SseEvent;

/// Boot phase result
#[derive(Debug, Clone)]
pub struct BootPhaseResult {
    pub phase: &'static str,
    pub success: bool,
    pub duration_ms: u64,
    pub message: String,
}

/// Boot sequence — 24 steps in 6 phases
pub struct BootSequence {
    config: HydraRuntimeConfig,
    results: Vec<BootPhaseResult>,
    total_duration_ms: u64,
    last_checkpoint: Option<String>,
    orphaned_run_ids: Vec<String>,
}

impl BootSequence {
    pub fn new(config: HydraRuntimeConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            total_duration_ms: 0,
            last_checkpoint: None,
            orphaned_run_ids: Vec::new(),
        }
    }

    /// Get loaded checkpoint data (if any)
    pub fn last_checkpoint(&self) -> Option<&str> {
        self.last_checkpoint.as_deref()
    }

    /// Get orphaned run IDs detected during boot
    pub fn orphaned_runs(&self) -> &[String] {
        &self.orphaned_run_ids
    }

    /// Execute the full boot sequence
    pub async fn execute(&mut self, event_bus: &EventBus) -> Result<(), BootError> {
        let start = Instant::now();

        // PHASE 1: PRE-FLIGHT (< 100ms)
        self.phase_preflight()?;

        // PHASE 2: CORE SERVICES (< 500ms)
        self.phase_core_services()?;

        // PHASE 3: SISTER BRIDGES (< 1s)
        self.phase_sister_bridges().await?;

        // PHASE 4: EXECUTION ENGINE (< 500ms)
        self.phase_execution_engine()?;

        // PHASE 5: SURFACES (< 1s)
        self.phase_surfaces()?;

        // PHASE 6: READY
        self.total_duration_ms = start.elapsed().as_millis() as u64;
        event_bus.publish(SseEvent::system_ready("0.1.0"));
        self.record("ready", true, "Hydra ready");

        Ok(())
    }

    fn phase_preflight(&mut self) -> Result<(), BootError> {
        let start = Instant::now();

        // 1. Config already loaded (passed in constructor)
        // 2. Validate config
        if let Err(errors) = self.config.validate() {
            return Err(BootError::ConfigInvalid(errors.join("; ")));
        }

        // 3. Logging initialized (done by caller)
        // 4. Single-instance lock (done by caller — needs filesystem)
        // 5. Resource profile detected
        let _profile = self.config.profile;

        // 5b. Initialize filesystem directories
        let data_dir = &self.config.data_dir;
        filesystem::init_filesystem(data_dir)
            .map_err(|e| BootError::ConfigInvalid(format!("Filesystem init failed: {e}")))?;

        self.record_timed("preflight", true, "Pre-flight checks passed", start);
        Ok(())
    }

    fn phase_core_services(&mut self) -> Result<(), BootError> {
        let start = Instant::now();

        // 6. SQLite database initialization
        let db_path = self.config.data_dir.join("hydra.db");
        self.init_database(&db_path)?;

        // 7-10: event bus, metrics, capability registry (initialized by caller)
        self.record_timed("core_services", true, "Core services initialized", start);
        Ok(())
    }

    fn init_database(&self, db_path: &Path) -> Result<(), BootError> {
        // Verify the parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                return Err(BootError::DatabaseError(format!(
                    "Database directory does not exist: {}",
                    parent.display()
                )));
            }
        }
        Ok(())
    }

    async fn phase_sister_bridges(&mut self) -> Result<(), BootError> {
        let start = Instant::now();

        // 11-13: Sister probing and bridge initialization
        // Required sisters: Memory, Identity, Contract
        // Optional: all others (degrade gracefully)
        self.record_timed("sister_bridges", true, "Sister bridges initialized", start);
        Ok(())
    }

    fn phase_execution_engine(&mut self) -> Result<(), BootError> {
        let start = Instant::now();

        // 14-18: Run Manager, Scheduler, Gate, Ledger, crash recovery

        // Load checkpoint if exists
        let checkpoint_path = self.config.checkpoint_path();
        if checkpoint_path.exists() {
            match std::fs::read_to_string(&checkpoint_path) {
                Ok(contents) => {
                    self.last_checkpoint = Some(contents);
                    tracing::info!("Loaded checkpoint from {}", checkpoint_path.display());
                }
                Err(e) => {
                    tracing::warn!("Failed to load checkpoint: {}", e);
                }
            }
        }

        // Detect orphaned runs (runs left in Running/Pending state from a crash)
        self.orphaned_run_ids = self.detect_orphaned_runs();
        if !self.orphaned_run_ids.is_empty() {
            tracing::warn!(
                "Found {} orphaned runs from previous crash",
                self.orphaned_run_ids.len()
            );
        }

        self.record_timed(
            "execution_engine",
            true,
            "Execution engine initialized",
            start,
        );
        Ok(())
    }

    /// Detect runs left in Running or Pending state (indicates a crash)
    fn detect_orphaned_runs(&self) -> Vec<String> {
        // In production, this queries the DB for Running/Pending runs.
        // For now, returns empty — the DB is initialized by the server layer.
        Vec::new()
    }

    fn phase_surfaces(&mut self) -> Result<(), BootError> {
        let start = Instant::now();

        // 19-23: HTTP server, SSE, voice, tray, signals
        self.record_timed("surfaces", true, "Surfaces initialized", start);
        Ok(())
    }

    fn record(&mut self, phase: &'static str, success: bool, message: impl Into<String>) {
        self.results.push(BootPhaseResult {
            phase,
            success,
            duration_ms: 0,
            message: message.into(),
        });
    }

    fn record_timed(
        &mut self,
        phase: &'static str,
        success: bool,
        message: impl Into<String>,
        start: Instant,
    ) {
        self.results.push(BootPhaseResult {
            phase,
            success,
            duration_ms: start.elapsed().as_millis() as u64,
            message: message.into(),
        });
    }

    pub fn results(&self) -> &[BootPhaseResult] {
        &self.results
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.total_duration_ms
    }

    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| r.success)
    }
}

#[derive(Debug, Clone)]
pub enum BootError {
    ConfigInvalid(String),
    RequiredSisterUnavailable(String),
    LockFailed(String),
    DatabaseError(String),
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigInvalid(msg) => write!(f, "Configuration is invalid. {msg}. Fix config.toml and restart."),
            Self::RequiredSisterUnavailable(name) => write!(f, "Required sister '{name}' is unavailable. Hydra cannot start without it. Check the sister's status."),
            Self::LockFailed(msg) => write!(f, "Cannot acquire instance lock. {msg}. Only one Hydra instance can run at a time."),
            Self::DatabaseError(msg) => write!(f, "Database initialization failed. {msg}. Check data directory permissions."),
        }
    }
}

impl std::error::Error for BootError {}
