use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::schema::{CREATE_TABLES, SCHEMA_VERSION};

// ═══════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Expired => "expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "denied" => Some(Self::Denied),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRow {
    pub id: String,
    pub intent: String,
    pub status: RunStatus,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub parent_run_id: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRow {
    pub id: String,
    pub run_id: String,
    pub sequence: i32,
    pub description: String,
    pub status: StepStatus,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub evidence_refs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRow {
    pub id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub created_at: String,
    pub state_snapshot: Vec<u8>,
    pub rollback_commands: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRow {
    pub id: String,
    pub run_id: String,
    pub action: String,
    pub target: Option<String>,
    pub risk_score: f64,
    pub created_at: String,
    pub expires_at: String,
    pub status: ApprovalStatus,
}

// ═══════════════════════════════════════════════════════════
// ERROR
// ═══════════════════════════════════════════════════════════

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Invalid status: {0}")]
    InvalidStatus(String),
}

// ═══════════════════════════════════════════════════════════
// DATABASE
// ═══════════════════════════════════════════════════════════

pub struct HydraDb {
    conn: Arc<Mutex<Connection>>,
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
        conn.execute_batch(CREATE_TABLES)?;

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
        conn.execute_batch(CREATE_TABLES)?;
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
            // Future migrations go here
            conn.execute(
                "UPDATE schema_version SET version = ?1",
                params![SCHEMA_VERSION],
            )?;
        }
        Ok(())
    }

    /// Get schema version
    pub fn schema_version(&self) -> Result<u32, DbError> {
        let conn = self.conn.lock();
        let v: u32 = conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
            row.get(0)
        })?;
        Ok(v)
    }

    // ═══════════════════════════════════════════════════════
    // RUNS
    // ═══════════════════════════════════════════════════════

    pub fn create_run(&self, run: &RunRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO runs (id, intent, status, created_at, updated_at, completed_at, parent_run_id, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![run.id, run.intent, run.status.as_str(), run.created_at, run.updated_at, run.completed_at, run.parent_run_id, run.metadata],
        )?;
        Ok(())
    }

    pub fn get_run(&self, id: &str) -> Result<RunRow, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, intent, status, created_at, updated_at, completed_at, parent_run_id, metadata FROM runs WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(2)?;
                Ok(RunRow {
                    id: row.get(0)?,
                    intent: row.get(1)?,
                    status: RunStatus::from_str(&status_str).unwrap_or(RunStatus::Pending),
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    completed_at: row.get(5)?,
                    parent_run_id: row.get(6)?,
                    metadata: row.get(7)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Run {id}")),
            other => DbError::Sqlite(other),
        })
    }

    pub fn update_run_status(
        &self,
        id: &str,
        status: RunStatus,
        completed_at: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE runs SET status = ?1, updated_at = ?2, completed_at = ?3 WHERE id = ?4",
            params![status.as_str(), now, completed_at, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Run {id}")));
        }
        Ok(())
    }

    pub fn list_runs(&self, status: Option<RunStatus>) -> Result<Vec<RunRow>, DbError> {
        let conn = self.conn.lock();
        let mut rows = Vec::new();
        if let Some(s) = status {
            let mut stmt = conn.prepare("SELECT id, intent, status, created_at, updated_at, completed_at, parent_run_id, metadata FROM runs WHERE status = ?1 ORDER BY created_at DESC")?;
            let iter = stmt.query_map(params![s.as_str()], |row| {
                let status_str: String = row.get(2)?;
                Ok(RunRow {
                    id: row.get(0)?,
                    intent: row.get(1)?,
                    status: RunStatus::from_str(&status_str).unwrap_or(RunStatus::Pending),
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    completed_at: row.get(5)?,
                    parent_run_id: row.get(6)?,
                    metadata: row.get(7)?,
                })
            })?;
            for r in iter {
                rows.push(r?);
            }
        } else {
            let mut stmt = conn.prepare("SELECT id, intent, status, created_at, updated_at, completed_at, parent_run_id, metadata FROM runs ORDER BY created_at DESC")?;
            let iter = stmt.query_map([], |row| {
                let status_str: String = row.get(2)?;
                Ok(RunRow {
                    id: row.get(0)?,
                    intent: row.get(1)?,
                    status: RunStatus::from_str(&status_str).unwrap_or(RunStatus::Pending),
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    completed_at: row.get(5)?,
                    parent_run_id: row.get(6)?,
                    metadata: row.get(7)?,
                })
            })?;
            for r in iter {
                rows.push(r?);
            }
        }
        Ok(rows)
    }

    pub fn delete_run(&self, id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute("DELETE FROM runs WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Run {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // STEPS
    // ═══════════════════════════════════════════════════════

    pub fn create_step(&self, step: &StepRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO steps (id, run_id, sequence, description, status, started_at, completed_at, result, evidence_refs) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![step.id, step.run_id, step.sequence, step.description, step.status.as_str(), step.started_at, step.completed_at, step.result, step.evidence_refs],
        )?;
        Ok(())
    }

    pub fn get_step(&self, id: &str) -> Result<StepRow, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, run_id, sequence, description, status, started_at, completed_at, result, evidence_refs FROM steps WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(4)?;
                Ok(StepRow {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    sequence: row.get(2)?,
                    description: row.get(3)?,
                    status: StepStatus::from_str(&status_str).unwrap_or(StepStatus::Pending),
                    started_at: row.get(5)?,
                    completed_at: row.get(6)?,
                    result: row.get(7)?,
                    evidence_refs: row.get(8)?,
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Step {id}")),
            other => DbError::Sqlite(other),
        })
    }

    pub fn list_steps(&self, run_id: &str) -> Result<Vec<StepRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, sequence, description, status, started_at, completed_at, result, evidence_refs FROM steps WHERE run_id = ?1 ORDER BY sequence"
        )?;
        let iter = stmt.query_map(params![run_id], |row| {
            let status_str: String = row.get(4)?;
            Ok(StepRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                sequence: row.get(2)?,
                description: row.get(3)?,
                status: StepStatus::from_str(&status_str).unwrap_or(StepStatus::Pending),
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                result: row.get(7)?,
                evidence_refs: row.get(8)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }

    pub fn update_step_status(
        &self,
        id: &str,
        status: StepStatus,
        completed_at: Option<&str>,
        result: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE steps SET status = ?1, completed_at = ?2, result = ?3 WHERE id = ?4",
            params![status.as_str(), completed_at, result, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Step {id}")));
        }
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // CHECKPOINTS
    // ═══════════════════════════════════════════════════════

    pub fn create_checkpoint(&self, cp: &CheckpointRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO checkpoints (id, run_id, step_id, created_at, state_snapshot, rollback_commands) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![cp.id, cp.run_id, cp.step_id, cp.created_at, cp.state_snapshot, cp.rollback_commands],
        )?;
        Ok(())
    }

    pub fn get_checkpoint(&self, id: &str) -> Result<CheckpointRow, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, run_id, step_id, created_at, state_snapshot, rollback_commands FROM checkpoints WHERE id = ?1",
            params![id],
            |row| Ok(CheckpointRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                step_id: row.get(2)?,
                created_at: row.get(3)?,
                state_snapshot: row.get(4)?,
                rollback_commands: row.get(5)?,
            }),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Checkpoint {id}")),
            other => DbError::Sqlite(other),
        })
    }

    pub fn list_checkpoints(&self, run_id: &str) -> Result<Vec<CheckpointRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step_id, created_at, state_snapshot, rollback_commands FROM checkpoints WHERE run_id = ?1 ORDER BY created_at"
        )?;
        let iter = stmt.query_map(params![run_id], |row| {
            Ok(CheckpointRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                step_id: row.get(2)?,
                created_at: row.get(3)?,
                state_snapshot: row.get(4)?,
                rollback_commands: row.get(5)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════
    // APPROVALS
    // ═══════════════════════════════════════════════════════

    pub fn create_approval(&self, a: &ApprovalRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO approvals (id, run_id, action, target, risk_score, created_at, expires_at, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.id, a.run_id, a.action, a.target, a.risk_score, a.created_at, a.expires_at, a.status.as_str()],
        )?;
        Ok(())
    }

    pub fn get_approval(&self, id: &str) -> Result<ApprovalRow, DbError> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, run_id, action, target, risk_score, created_at, expires_at, status FROM approvals WHERE id = ?1",
            params![id],
            |row| {
                let status_str: String = row.get(7)?;
                Ok(ApprovalRow {
                    id: row.get(0)?,
                    run_id: row.get(1)?,
                    action: row.get(2)?,
                    target: row.get(3)?,
                    risk_score: row.get(4)?,
                    created_at: row.get(5)?,
                    expires_at: row.get(6)?,
                    status: ApprovalStatus::from_str(&status_str).unwrap_or(ApprovalStatus::Pending),
                })
            },
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => DbError::NotFound(format!("Approval {id}")),
            other => DbError::Sqlite(other),
        })
    }

    pub fn update_approval_status(&self, id: &str, status: ApprovalStatus) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "UPDATE approvals SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound(format!("Approval {id}")));
        }
        Ok(())
    }

    pub fn list_pending_approvals(&self) -> Result<Vec<ApprovalRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, action, target, risk_score, created_at, expires_at, status FROM approvals WHERE status = 'pending' ORDER BY created_at"
        )?;
        let iter = stmt.query_map([], |row| {
            let status_str: String = row.get(7)?;
            Ok(ApprovalRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                action: row.get(2)?,
                target: row.get(3)?,
                risk_score: row.get(4)?,
                created_at: row.get(5)?,
                expires_at: row.get(6)?,
                status: ApprovalStatus::from_str(&status_str).unwrap_or(ApprovalStatus::Pending),
            })
        })?;
        let mut rows = Vec::new();
        for r in iter {
            rows.push(r?);
        }
        Ok(rows)
    }
}

impl Clone for HydraDb {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
