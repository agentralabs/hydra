pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_create_tables_contains_runs() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS runs"));
    }

    #[test]
    fn test_create_tables_contains_steps() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS steps"));
    }

    #[test]
    fn test_create_tables_contains_checkpoints() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS checkpoints"));
    }

    #[test]
    fn test_create_tables_contains_approvals() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS approvals"));
    }

    #[test]
    fn test_create_tables_contains_schema_version() {
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS schema_version"));
    }

    #[test]
    fn test_create_tables_has_foreign_keys() {
        assert!(CREATE_TABLES.contains("REFERENCES runs(id)"));
    }

    #[test]
    fn test_create_tables_has_indexes() {
        assert!(CREATE_TABLES.contains("CREATE INDEX IF NOT EXISTS"));
    }

    #[test]
    fn test_create_tables_has_status_checks() {
        assert!(CREATE_TABLES.contains("CHECK(status IN"));
    }
}

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

CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_runs_created ON runs(created_at);
CREATE INDEX IF NOT EXISTS idx_steps_run ON steps(run_id);
CREATE INDEX IF NOT EXISTS idx_approvals_pending ON approvals(status) WHERE status = 'pending';
"#;
