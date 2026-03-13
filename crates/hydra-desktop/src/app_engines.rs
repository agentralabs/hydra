// Single block expression returning a tuple of all engine handles.
// Used as: let (...) = include!("app_engines.rs");
{
    // ── Init graduated autonomy engine ──
    let decide_engine: Arc<DecideEngine> = use_hook(|| Arc::new(DecideEngine::new()));

    let invention_engine: Arc<InventionEngine> = use_hook(|| {
        let engine = Arc::new(InventionEngine::new());
        let inv = engine.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                inv.tick_idle(10);
                if let Some(dream_insights) = inv.maybe_dream() {
                    tracing::info!("[hydra] Dream insights: {}", dream_insights);
                }
            }
        });
        engine
    });
    let proactive_notifier: Arc<parking_lot::Mutex<ProactiveNotifier>> =
        use_hook(|| Arc::new(parking_lot::Mutex::new(ProactiveNotifier::new())));
    let agent_spawner: Arc<AgentSpawner> = use_hook(|| Arc::new(AgentSpawner::new(100)));
    let undo_stack: Arc<parking_lot::Mutex<UndoStack>> = use_hook(|| Arc::new(parking_lot::Mutex::new(UndoStack::new(100))));
    let approval_manager: Arc<ApprovalManager> = use_hook(|| Arc::new(ApprovalManager::with_default_timeout()));
    let federation_manager: Arc<FederationManager> = use_hook(|| Arc::new(FederationManager::new()));
    // ── Initialize security database ──
    let hydra_db: Option<Arc<HydraDb>> = use_hook(|| {
        let db_path = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(".hydra")
            .join("security.db");
        match HydraDb::init(&db_path) {
            Ok(db) => {
                tracing::info!("[hydra] Security DB initialized at {:?}", db_path);
                Some(Arc::new(db))
            }
            Err(e) => {
                tracing::warn!("[hydra] Failed to init security DB: {}", e);
                None
            }
        }
    });

    let swarm_manager: Arc<hydra_native::swarm::SwarmManager> = use_hook(|| {
        Arc::new(hydra_native::swarm::SwarmManager::default())
    });

    // ── Init file watcher for proactive suggestions (P2) ──
    let file_watcher: Option<Arc<parking_lot::Mutex<hydra_pulse::FileWatcher>>> = use_hook(|| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match hydra_pulse::FileWatcher::start(cwd) {
            Ok(watcher) => {
                tracing::info!("[hydra] File watcher started");
                Some(Arc::new(parking_lot::Mutex::new(watcher)))
            }
            Err(e) => {
                tracing::warn!("[hydra] File watcher failed to start: {}", e);
                None
            }
        }
    });
    let proactive_file_engine: Arc<parking_lot::Mutex<hydra_pulse::ProactiveFileEngine>> =
        use_hook(|| Arc::new(parking_lot::Mutex::new(hydra_pulse::ProactiveFileEngine::new())));

    (decide_engine, invention_engine, proactive_notifier, agent_spawner, undo_stack, approval_manager, federation_manager, hydra_db, swarm_manager, file_watcher, proactive_file_engine)
}
