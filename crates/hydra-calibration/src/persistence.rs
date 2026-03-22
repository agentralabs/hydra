//! SQLite persistence for calibration records.

use crate::record::CalibrationRecord;
use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed calibration store.
pub struct CalibrationDb {
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

impl CalibrationDb {
    /// Open the calibration database, creating the table if needed.
    pub fn open() -> rusqlite::Result<Self> {
        let path = data_dir().join("calibration.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS calibration_records (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                judgment_type TEXT NOT NULL,
                stated_confidence REAL NOT NULL,
                recorded_at TEXT NOT NULL,
                record_json TEXT NOT NULL
            )",
            [],
        )?;
        eprintln!("hydra: calibration db opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Insert a record. Ignores duplicates by ID.
    pub fn insert(&self, record: &CalibrationRecord) {
        let json = match serde_json::to_string(record) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("hydra: calibration serialize failed: {}", e);
                return;
            }
        };
        let recorded_at = record.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT OR IGNORE INTO calibration_records \
             (id, domain, judgment_type, stated_confidence, recorded_at, record_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                record.id,
                record.domain,
                record.judgment_type.label(),
                record.stated_confidence,
                recorded_at,
                json,
            ],
        ) {
            Ok(_) => eprintln!("hydra: calibration record persisted: {}", record.id),
            Err(e) => eprintln!("hydra: calibration insert failed: {}", e),
        }
    }

    /// Load all records from the database.
    pub fn load_all(&self) -> Vec<CalibrationRecord> {
        let mut stmt = match self
            .conn
            .prepare("SELECT record_json FROM calibration_records ORDER BY recorded_at")
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hydra: calibration load_all prepare failed: {}", e);
                return Vec::new();
            }
        };
        let rows = match stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hydra: calibration load_all query failed: {}", e);
                return Vec::new();
            }
        };
        let mut records = Vec::new();
        for row in rows.flatten() {
            match serde_json::from_str::<CalibrationRecord>(&row) {
                Ok(r) => records.push(r),
                Err(e) => eprintln!("hydra: calibration deserialize failed: {}", e),
            }
        }
        records
    }

    /// Count records in the database.
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM calibration_records", [], |row| {
                row.get::<_, usize>(0)
            })
            .unwrap_or(0)
    }
}
