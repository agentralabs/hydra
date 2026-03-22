//! SQLite persistence for settlement records.

use crate::record::SettlementRecord;
use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed settlement store.
pub struct SettlementDb {
    conn: Connection,
}

fn data_dir() -> PathBuf {
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hydra")
        .join("data");
    if !base.exists() {
        let _ = std::fs::create_dir_all(&base);
    }
    base
}

impl SettlementDb {
    /// Open the settlement database, creating the table if needed.
    pub fn open() -> rusqlite::Result<Self> {
        let path = data_dir().join("settlement.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settlement_records (
                id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL,
                settled_at TEXT NOT NULL
            )",
            [],
        )?;
        eprintln!("hydra: settlement db opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Insert a record. Ignores duplicates by ID.
    pub fn insert(&self, record: &SettlementRecord) {
        let json = match serde_json::to_string(record) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("hydra: settlement serialize failed: {}", e);
                return;
            }
        };
        let settled_at = record.settled_at.to_rfc3339();
        match self.conn.execute(
            "INSERT OR IGNORE INTO settlement_records (id, record_json, settled_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![record.id, json, settled_at],
        ) {
            Ok(_) => eprintln!("hydra: settlement record persisted: {}", record.id),
            Err(e) => eprintln!("hydra: settlement insert failed: {}", e),
        }
    }

    /// Load all records from the database.
    pub fn load_all(&self) -> Vec<SettlementRecord> {
        let mut stmt = match self
            .conn
            .prepare("SELECT record_json FROM settlement_records ORDER BY settled_at")
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hydra: settlement load_all prepare failed: {}", e);
                return Vec::new();
            }
        };
        let rows = match stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hydra: settlement load_all query failed: {}", e);
                return Vec::new();
            }
        };
        let mut records = Vec::new();
        for row in rows.flatten() {
            match serde_json::from_str::<SettlementRecord>(&row) {
                Ok(r) => records.push(r),
                Err(e) => eprintln!("hydra: settlement deserialize failed: {}", e),
            }
        }
        records
    }

    /// Count records in the database.
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM settlement_records", [], |row| {
                row.get::<_, usize>(0)
            })
            .unwrap_or(0)
    }
}
