use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::{params, Connection};

use crate::schema::SCHEMA_VERSION;
use crate::schema::CREATE_TABLES;
pub use crate::store_types::DbError;

// ═══════════════════════════════════════════════════════════
// DATABASE
// ═══════════════════════════════════════════════════════════

pub struct HydraDb {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl HydraDb {
    /// Initialize database at path (creates file and tables if needed)
    pub fn init(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch("PRAGMA busy_timeout=5000;")?;

        // Create tables
        conn.execute_batch(&*CREATE_TABLES)?;

        // Set schema version if not set
        let version: Option<u32> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .ok();
        if version.is_none() {
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![SCHEMA_VERSION],
            )?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Initialize in-memory database (for tests)
    pub fn in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(&*CREATE_TABLES)?;
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run pending migrations
    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let current: u32 = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        if current < SCHEMA_VERSION {
            // V1→V2: Add intelligence tables (outcome_history, calibration_buckets, user_profile_learned)
            if current < 2 {
                conn.execute_batch(crate::schema::CREATE_INTELLIGENCE_TABLES)?;
            }
            // V2→V3: Add code index tables (code_symbols, code_edges)
            if current < 3 {
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS code_symbols (
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
                    CREATE INDEX IF NOT EXISTS idx_code_edges_to ON code_edges(to_symbol);"
                )?;
            }
            conn.execute(
                "UPDATE schema_version SET version = ?1",
                params![SCHEMA_VERSION],
            )?;
        }
        Ok(())
    }
}

impl Clone for HydraDb {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
