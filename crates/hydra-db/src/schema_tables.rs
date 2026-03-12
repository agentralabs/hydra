pub const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    intent TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','running','paused','completed','failed','cancelled')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    parent_run_id TEXT REFERENCES runs(id),
    metadata TEXT
);

CREATE TABLE IF NOT EXISTS steps (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','running','completed','failed','skipped')),
    started_at TEXT,
    completed_at TEXT,
    result TEXT,
    evidence_refs TEXT
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id TEXT REFERENCES steps(id),
    created_at TEXT NOT NULL,
    state_snapshot BLOB NOT NULL,
    rollback_commands TEXT
);

CREATE TABLE IF NOT EXISTS approvals (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    target TEXT,
    risk_score REAL NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','approved','denied','expired'))
);

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    actions TEXT NOT NULL,
    trigger_pattern TEXT,
    confidence REAL DEFAULT 0.5,
    executions INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    actions TEXT NOT NULL,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    avg_duration_ms REAL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS reflections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reflection_type TEXT NOT NULL,
    content TEXT NOT NULL,
    severity REAL DEFAULT 0.0,
    suggestion TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS trust_scores (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL,
    score REAL NOT NULL DEFAULT 0.5,
    total_actions INTEGER DEFAULT 0,
    successful_actions INTEGER DEFAULT 0,
    failed_actions INTEGER DEFAULT 0,
    autonomy_level TEXT NOT NULL DEFAULT 'Observer',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS dream_insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    surfaced INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS shadow_validations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_description TEXT NOT NULL,
    safe INTEGER NOT NULL DEFAULT 1,
    divergence_count INTEGER DEFAULT 0,
    critical_divergences INTEGER DEFAULT 0,
    recommendation TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS predictions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_name TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    risk_recommendation TEXT,
    description TEXT,
    actual_outcome INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS proactive_alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    priority TEXT NOT NULL DEFAULT 'Low',
    delivered INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS agent_sessions (
    id TEXT PRIMARY KEY,
    parent_task TEXT NOT NULL,
    subtask_count INTEGER NOT NULL DEFAULT 0,
    completed_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS federation_peers (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL,
    trust_level REAL NOT NULL DEFAULT 0.5,
    capabilities TEXT,
    last_seen TEXT NOT NULL DEFAULT (datetime('now')),
    active_tasks INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS temporal_memories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS compression_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_tokens INTEGER NOT NULL,
    compressed_tokens INTEGER NOT NULL,
    ratio REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS mutation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_name TEXT NOT NULL,
    mutation_type TEXT NOT NULL,
    fitness_before REAL,
    fitness_after REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS evolution_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    generation INTEGER NOT NULL,
    patterns_count INTEGER NOT NULL,
    best_fitness REAL NOT NULL,
    avg_fitness REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS receipts (
    id TEXT PRIMARY KEY,
    receipt_type TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL DEFAULT 'hydra',
    tokens_used INTEGER DEFAULT 0,
    risk_level TEXT,
    hash TEXT NOT NULL,
    prev_hash TEXT,
    sequence INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS budget_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT,
    phase TEXT NOT NULL,
    tokens_spent INTEGER NOT NULL,
    tokens_remaining INTEGER NOT NULL,
    conservation_mode INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_runs_created ON runs(created_at);
CREATE INDEX IF NOT EXISTS idx_steps_run ON steps(run_id);
CREATE INDEX IF NOT EXISTS idx_approvals_pending ON approvals(status) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
CREATE INDEX IF NOT EXISTS idx_patterns_name ON patterns(name);
CREATE INDEX IF NOT EXISTS idx_reflections_type ON reflections(reflection_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trust_domain ON trust_scores(domain);
CREATE INDEX IF NOT EXISTS idx_dreams_category ON dream_insights(category);
CREATE INDEX IF NOT EXISTS idx_alerts_priority ON proactive_alerts(priority);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_status ON agent_sessions(status);
CREATE INDEX IF NOT EXISTS idx_temporal_category ON temporal_memories(category);
CREATE INDEX IF NOT EXISTS idx_compression_created ON compression_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_receipts_type ON receipts(receipt_type);
CREATE INDEX IF NOT EXISTS idx_receipts_sequence ON receipts(sequence);
CREATE INDEX IF NOT EXISTS idx_budget_run ON budget_usage(run_id);
CREATE INDEX IF NOT EXISTS idx_evolution_gen ON evolution_log(generation);

CREATE TABLE IF NOT EXISTS anomaly_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    command TEXT NOT NULL,
    detail TEXT,
    severity TEXT NOT NULL DEFAULT 'medium',
    kill_switch_engaged INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_anomaly_type ON anomaly_events(event_type);
CREATE INDEX IF NOT EXISTS idx_anomaly_severity ON anomaly_events(severity);

CREATE TABLE IF NOT EXISTS cursor_sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    mode TEXT NOT NULL DEFAULT 'visible',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    event_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS cursor_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES cursor_sessions(id) ON DELETE CASCADE,
    timestamp_ms INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    x REAL NOT NULL DEFAULT 0,
    y REAL NOT NULL DEFAULT 0,
    payload TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cursor_sessions_task ON cursor_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_cursor_events_session ON cursor_events(session_id);
CREATE INDEX IF NOT EXISTS idx_cursor_events_ts ON cursor_events(timestamp_ms);

CREATE TABLE IF NOT EXISTS beliefs (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL CHECK(category IN ('preference','fact','convention','correction')),
    subject TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    source TEXT NOT NULL CHECK(source IN ('user_stated','inferred','corrected')),
    confirmations INTEGER NOT NULL DEFAULT 0,
    contradictions INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1,
    supersedes TEXT,
    superseded_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_beliefs_subject ON beliefs(subject);
CREATE INDEX IF NOT EXISTS idx_beliefs_category ON beliefs(category);
CREATE INDEX IF NOT EXISTS idx_beliefs_active ON beliefs(active) WHERE active = 1;

CREATE TABLE IF NOT EXISTS mcp_discovered_skills (
    id TEXT PRIMARY KEY,
    server_name TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    description TEXT,
    input_schema TEXT,
    discovered_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    use_count INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_mcp_skills_server ON mcp_discovered_skills(server_name);
CREATE INDEX IF NOT EXISTS idx_mcp_skills_active ON mcp_discovered_skills(active) WHERE active = 1;

CREATE TABLE IF NOT EXISTS federation_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id TEXT NOT NULL,
    peer_name TEXT,
    endpoint TEXT NOT NULL,
    trust_level TEXT NOT NULL DEFAULT 'unknown',
    capabilities TEXT,
    federation_type TEXT NOT NULL DEFAULT 'personal',
    last_sync_version INTEGER NOT NULL DEFAULT 0,
    last_seen TEXT NOT NULL DEFAULT (datetime('now')),
    active_tasks INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_federation_peer ON federation_state(peer_id);
CREATE INDEX IF NOT EXISTS idx_federation_active ON federation_state(active) WHERE active = 1;

CREATE TABLE IF NOT EXISTS repair_runs (
    id TEXT PRIMARY KEY,
    spec_file TEXT NOT NULL,
    task TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','success','failed','escalated')),
    iteration INTEGER NOT NULL DEFAULT 0,
    max_iterations INTEGER NOT NULL DEFAULT 5,
    checks_total INTEGER NOT NULL DEFAULT 0,
    checks_passed INTEGER NOT NULL DEFAULT 0,
    failure_log TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    duration_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_repair_status ON repair_runs(status);
CREATE INDEX IF NOT EXISTS idx_repair_spec ON repair_runs(spec_file);

CREATE TABLE IF NOT EXISTS repair_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES repair_runs(id) ON DELETE CASCADE,
    iteration INTEGER NOT NULL,
    check_name TEXT NOT NULL,
    check_command TEXT NOT NULL,
    passed INTEGER NOT NULL DEFAULT 0,
    output TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_repair_checks_run ON repair_checks(run_id);
"#;
