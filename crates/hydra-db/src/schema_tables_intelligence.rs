/// Intelligence persistence tables — outcome tracking, calibration, user traits.
///
/// Separated from schema_tables.rs to stay under 400-line limit.
/// Appended to CREATE_TABLES via schema.rs.
pub const CREATE_INTELLIGENCE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS outcome_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    intent_category TEXT NOT NULL,
    topic TEXT NOT NULL DEFAULT '',
    model_used TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('success','correction','failure','repeat','neutral')),
    tokens_used INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS calibration_buckets (
    bucket_index INTEGER PRIMARY KEY CHECK(bucket_index BETWEEN 0 AND 9),
    total INTEGER NOT NULL DEFAULT 0,
    successes INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_profile_learned (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trait_key TEXT NOT NULL UNIQUE,
    trait_value TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    observation_count INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_outcome_category ON outcome_history(intent_category);
CREATE INDEX IF NOT EXISTS idx_outcome_created ON outcome_history(created_at);
CREATE INDEX IF NOT EXISTS idx_user_trait ON user_profile_learned(trait_key);

CREATE TABLE IF NOT EXISTS code_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    symbol_name TEXT NOT NULL,
    symbol_type TEXT NOT NULL CHECK(symbol_type IN ('function','struct','enum','trait','impl','const','type','mod','macro')),
    line_number INTEGER NOT NULL DEFAULT 0,
    visibility TEXT NOT NULL DEFAULT 'private',
    signature TEXT,
    doc_comment TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS code_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_symbol TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK(edge_type IN ('calls','implements','uses','imports','contains')),
    file_path TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_code_symbols_file ON code_symbols(file_path);
CREATE INDEX IF NOT EXISTS idx_code_symbols_name ON code_symbols(symbol_name);
CREATE INDEX IF NOT EXISTS idx_code_symbols_type ON code_symbols(symbol_type);
CREATE INDEX IF NOT EXISTS idx_code_edges_from ON code_edges(from_symbol);
CREATE INDEX IF NOT EXISTS idx_code_edges_to ON code_edges(to_symbol);
"#;
