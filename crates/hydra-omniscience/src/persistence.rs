//! SQLite persistence for knowledge gaps.

use crate::gap::KnowledgeGap;
use rusqlite::Connection;
use std::path::PathBuf;

/// SQLite-backed omniscience store.
pub struct OmniscienceDb {
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

impl OmniscienceDb {
    /// Open the omniscience database, creating the table if needed.
    pub fn open() -> rusqlite::Result<Self> {
        let path = data_dir().join("omniscience.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS knowledge_gaps (
                id TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                gap_type TEXT NOT NULL,
                priority REAL NOT NULL,
                state TEXT NOT NULL,
                detected_at TEXT NOT NULL,
                gap_json TEXT NOT NULL
            )",
            [],
        )?;
        eprintln!("hydra: omniscience db opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Insert a gap. Ignores duplicates by ID.
    pub fn insert(&self, gap: &KnowledgeGap) {
        let json = match serde_json::to_string(gap) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("hydra: omniscience serialize failed: {}", e);
                return;
            }
        };
        let detected_at = gap.detected_at.to_rfc3339();
        match self.conn.execute(
            "INSERT OR IGNORE INTO knowledge_gaps \
             (id, topic, gap_type, priority, state, detected_at, gap_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                gap.id,
                gap.topic,
                gap.gap_type.label(),
                gap.priority,
                gap.state.label(),
                detected_at,
                json,
            ],
        ) {
            Ok(_) => eprintln!("hydra: omniscience gap persisted: {}", gap.id),
            Err(e) => eprintln!("hydra: omniscience insert failed: {}", e),
        }
    }

    /// Load all gaps from the database.
    pub fn load_all(&self) -> Vec<KnowledgeGap> {
        let mut stmt = match self
            .conn
            .prepare("SELECT gap_json FROM knowledge_gaps ORDER BY detected_at")
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("hydra: omniscience load_all prepare failed: {}", e);
                return Vec::new();
            }
        };
        let rows = match stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hydra: omniscience load_all query failed: {}", e);
                return Vec::new();
            }
        };
        let mut gaps = Vec::new();
        for row in rows.flatten() {
            match serde_json::from_str::<KnowledgeGap>(&row) {
                Ok(g) => gaps.push(g),
                Err(e) => eprintln!("hydra: omniscience deserialize failed: {}", e),
            }
        }
        gaps
    }

    /// Count gaps in the database.
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM knowledge_gaps", [], |row| {
                row.get::<_, usize>(0)
            })
            .unwrap_or(0)
    }
}
