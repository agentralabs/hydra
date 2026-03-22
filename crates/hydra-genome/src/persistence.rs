//! SQLite persistence for genome entries.

use crate::entry::GenomeEntry;
use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed genome store.
pub struct GenomeDb {
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

impl GenomeDb {
    /// Open the genome database, creating the table if needed.
    pub fn open() -> rusqlite::Result<Self> {
        let path = data_dir().join("genome.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS genome_entries (
                id TEXT PRIMARY KEY,
                entry_json TEXT NOT NULL,
                added_at TEXT NOT NULL
            )",
            [],
        )?;
        eprintln!("hydra: genome db opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Insert an entry. Ignores duplicates by ID.
    pub fn insert(&self, entry: &GenomeEntry) {
        let json = match serde_json::to_string(entry) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("hydra: genome serialize failed: {}", e);
                return;
            }
        };
        let added_at = entry.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT OR IGNORE INTO genome_entries (id, entry_json, added_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![entry.id, json, added_at],
        ) {
            Ok(_) => eprintln!("hydra: genome entry persisted: {}", entry.id),
            Err(e) => eprintln!("hydra: genome insert failed: {}", e),
        }
    }

    /// Load all entries from the database.
    pub fn load_all(&self) -> Vec<GenomeEntry> {
        let mut stmt = match self
            .conn
            .prepare("SELECT entry_json FROM genome_entries ORDER BY added_at")
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hydra: genome load_all prepare failed: {}", e);
                return Vec::new();
            }
        };
        let rows = match stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hydra: genome load_all query failed: {}", e);
                return Vec::new();
            }
        };
        let mut entries = Vec::new();
        for json in rows.flatten() {
            match serde_json::from_str::<GenomeEntry>(&json) {
                Ok(e) => entries.push(e),
                Err(e) => eprintln!("hydra: genome deserialize failed: {}", e),
            }
        }
        entries
    }

    /// Count entries in the database.
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM genome_entries", [], |row| {
                row.get::<_, usize>(0)
            })
            .unwrap_or(0)
    }
}
