//! SQLite persistence for wisdom memory entries.

use crate::memory::WisdomMemoryEntry;
use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed wisdom store.
pub struct WisdomDb {
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

impl WisdomDb {
    /// Open the wisdom database, creating the table if needed.
    pub fn open() -> rusqlite::Result<Self> {
        let path = data_dir().join("wisdom.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS wisdom_entries (
                id TEXT PRIMARY KEY,
                entry_json TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        eprintln!("hydra: wisdom db opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Insert an entry. Ignores duplicates by ID.
    pub fn insert(&self, entry: &WisdomMemoryEntry) {
        let json = match serde_json::to_string(entry) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("hydra: wisdom serialize failed: {}", e);
                return;
            }
        };
        let created_at = entry.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT OR IGNORE INTO wisdom_entries (id, entry_json, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![entry.id, json, created_at],
        ) {
            Ok(_) => eprintln!("hydra: wisdom entry persisted: {}", entry.id),
            Err(e) => eprintln!("hydra: wisdom insert failed: {}", e),
        }
    }

    /// Load all entries from the database.
    pub fn load_all(&self) -> Vec<WisdomMemoryEntry> {
        let mut stmt = match self
            .conn
            .prepare("SELECT entry_json FROM wisdom_entries ORDER BY created_at")
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hydra: wisdom load_all prepare failed: {}", e);
                return Vec::new();
            }
        };
        let rows = match stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hydra: wisdom load_all query failed: {}", e);
                return Vec::new();
            }
        };
        let mut entries = Vec::new();
        for row in rows.flatten() {
            match serde_json::from_str::<WisdomMemoryEntry>(&row) {
                Ok(e) => entries.push(e),
                Err(e) => eprintln!("hydra: wisdom deserialize failed: {}", e),
            }
        }
        entries
    }

    /// Count entries in the database.
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM wisdom_entries", [], |row| {
                row.get::<_, usize>(0)
            })
            .unwrap_or(0)
    }
}
