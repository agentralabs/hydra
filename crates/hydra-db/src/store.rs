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
pub struct ReceiptRow {
    pub id: String,
    pub receipt_type: String,
    pub action: String,
    pub actor: String,
    pub tokens_used: i64,
    pub risk_level: Option<String>,
    pub hash: String,
    pub prev_hash: Option<String>,
    pub sequence: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowValidationRow {
    pub action_description: String,
    pub safe: bool,
    pub divergence_count: i32,
    pub critical_divergences: i32,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEventRow {
    pub event_type: String,
    pub command: String,
    pub detail: Option<String>,
    pub severity: String,
    pub kill_switch_engaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreRow {
    pub domain: String,
    pub score: f64,
    pub total_actions: i64,
    pub successful_actions: i64,
    pub failed_actions: i64,
    pub autonomy_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSessionRow {
    pub id: String,
    pub task_id: String,
    pub mode: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub event_count: i64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorEventRow {
    pub timestamp_ms: i64,
    pub event_type: String,
    pub x: f64,
    pub y: f64,
    pub payload: Option<String>,
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

    /// Expose the shared connection for subsystems (e.g. MessageStore)
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    // ═══════════════════════════════════════════════════════
    // RECEIPTS
    // ═══════════════════════════════════════════════════════

    pub fn create_receipt(&self, r: &ReceiptRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO receipts (id, receipt_type, action, actor, tokens_used, risk_level, hash, prev_hash, sequence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![r.id, r.receipt_type, r.action, r.actor, r.tokens_used, r.risk_level, r.hash, r.prev_hash, r.sequence, r.created_at],
        )?;
        Ok(())
    }

    pub fn list_receipts(&self, limit: usize) -> Result<Vec<ReceiptRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, receipt_type, action, actor, tokens_used, risk_level, hash, prev_hash, sequence, created_at FROM receipts ORDER BY sequence DESC LIMIT ?1"
        )?;
        let iter = stmt.query_map(params![limit as i64], |row| {
            Ok(ReceiptRow {
                id: row.get(0)?,
                receipt_type: row.get(1)?,
                action: row.get(2)?,
                actor: row.get(3)?,
                tokens_used: row.get(4)?,
                risk_level: row.get(5)?,
                hash: row.get(6)?,
                prev_hash: row.get(7)?,
                sequence: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    pub fn last_receipt_hash(&self) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock();
        let result: Option<String> = conn.query_row(
            "SELECT hash FROM receipts ORDER BY sequence DESC LIMIT 1", [], |row| row.get(0),
        ).ok();
        Ok(result)
    }

    pub fn next_receipt_sequence(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock();
        let max: Option<i64> = conn.query_row(
            "SELECT MAX(sequence) FROM receipts", [], |row| row.get(0),
        ).ok().flatten();
        Ok(max.unwrap_or(0) + 1)
    }

    // ═══════════════════════════════════════════════════════
    // SHADOW VALIDATIONS
    // ═══════════════════════════════════════════════════════

    pub fn create_shadow_validation(&self, sv: &ShadowValidationRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO shadow_validations (action_description, safe, divergence_count, critical_divergences, recommendation) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sv.action_description, sv.safe as i32, sv.divergence_count, sv.critical_divergences, sv.recommendation],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════
    // ANOMALY EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn create_anomaly_event(&self, ae: &AnomalyEventRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO anomaly_events (event_type, command, detail, severity, kill_switch_engaged) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ae.event_type, ae.command, ae.detail, ae.severity, ae.kill_switch_engaged as i32],
        )?;
        Ok(())
    }

    pub fn list_anomaly_events(&self, limit: usize) -> Result<Vec<AnomalyEventRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT event_type, command, detail, severity, kill_switch_engaged FROM anomaly_events ORDER BY created_at DESC LIMIT ?1"
        )?;
        let iter = stmt.query_map(params![limit as i64], |row| {
            let ks: i32 = row.get(4)?;
            Ok(AnomalyEventRow {
                event_type: row.get(0)?,
                command: row.get(1)?,
                detail: row.get(2)?,
                severity: row.get(3)?,
                kill_switch_engaged: ks != 0,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════
    // TRUST SCORES
    // ═══════════════════════════════════════════════════════

    pub fn upsert_trust_score(&self, ts: &TrustScoreRow) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO trust_scores (domain, score, total_actions, successful_actions, failed_actions, autonomy_level, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(domain) DO UPDATE SET score=?2, total_actions=?3, successful_actions=?4, failed_actions=?5, autonomy_level=?6, updated_at=?7",
            params![ts.domain, ts.score, ts.total_actions, ts.successful_actions, ts.failed_actions, ts.autonomy_level, now],
        )?;
        Ok(())
    }

    pub fn get_trust_score(&self, domain: &str) -> Result<Option<TrustScoreRow>, DbError> {
        let conn = self.conn.lock();
        let result = conn.query_row(
            "SELECT domain, score, total_actions, successful_actions, failed_actions, autonomy_level FROM trust_scores WHERE domain = ?1",
            params![domain],
            |row| Ok(TrustScoreRow {
                domain: row.get(0)?,
                score: row.get(1)?,
                total_actions: row.get(2)?,
                successful_actions: row.get(3)?,
                failed_actions: row.get(4)?,
                autonomy_level: row.get(5)?,
            }),
        );
        match result {
            Ok(ts) => Ok(Some(ts)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }

    // ═══════════════════════════════════════════════════════
    // CURSOR SESSIONS & EVENTS
    // ═══════════════════════════════════════════════════════

    pub fn create_cursor_session(&self, id: &str, task_id: &str, mode: &str) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO cursor_sessions (id, task_id, mode, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, task_id, mode, now],
        )?;
        Ok(())
    }

    pub fn finish_cursor_session(&self, id: &str, event_count: i64, duration_ms: i64) -> Result<(), DbError> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE cursor_sessions SET ended_at = ?1, event_count = ?2, total_duration_ms = ?3 WHERE id = ?4",
            params![now, event_count, duration_ms, id],
        )?;
        Ok(())
    }

    pub fn record_cursor_event(
        &self,
        session_id: &str,
        timestamp_ms: i64,
        event_type: &str,
        x: f64,
        y: f64,
        payload: Option<&str>,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO cursor_events (session_id, timestamp_ms, event_type, x, y, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, timestamp_ms, event_type, x, y, payload],
        )?;
        Ok(())
    }

    pub fn list_cursor_sessions(&self, task_id: Option<&str>, limit: usize) -> Result<Vec<CursorSessionRow>, DbError> {
        let conn = self.conn.lock();
        let mut rows = Vec::new();
        if let Some(tid) = task_id {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, mode, started_at, ended_at, event_count, total_duration_ms FROM cursor_sessions WHERE task_id = ?1 ORDER BY started_at DESC LIMIT ?2"
            )?;
            let iter = stmt.query_map(params![tid, limit as i64], |row| {
                Ok(CursorSessionRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    mode: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    event_count: row.get(5)?,
                    total_duration_ms: row.get(6)?,
                })
            })?;
            for r in iter { rows.push(r?); }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, mode, started_at, ended_at, event_count, total_duration_ms FROM cursor_sessions ORDER BY started_at DESC LIMIT ?1"
            )?;
            let iter = stmt.query_map(params![limit as i64], |row| {
                Ok(CursorSessionRow {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    mode: row.get(2)?,
                    started_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    event_count: row.get(5)?,
                    total_duration_ms: row.get(6)?,
                })
            })?;
            for r in iter { rows.push(r?); }
        }
        Ok(rows)
    }

    pub fn get_cursor_events(&self, session_id: &str) -> Result<Vec<CursorEventRow>, DbError> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT timestamp_ms, event_type, x, y, payload FROM cursor_events WHERE session_id = ?1 ORDER BY timestamp_ms"
        )?;
        let iter = stmt.query_map(params![session_id], |row| {
            Ok(CursorEventRow {
                timestamp_ms: row.get(0)?,
                event_type: row.get(1)?,
                x: row.get(2)?,
                y: row.get(3)?,
                payload: row.get(4)?,
            })
        })?;
        let mut rows = Vec::new();
        for r in iter { rows.push(r?); }
        Ok(rows)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_run(id: &str, intent: &str, status: RunStatus) -> RunRow {
        let now = Utc::now().to_rfc3339();
        RunRow {
            id: id.into(),
            intent: intent.into(),
            status,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
            parent_run_id: None,
            metadata: None,
        }
    }

    fn make_step(id: &str, run_id: &str, seq: i32) -> StepRow {
        StepRow {
            id: id.into(),
            run_id: run_id.into(),
            sequence: seq,
            description: format!("Step {}", seq),
            status: StepStatus::Pending,
            started_at: None,
            completed_at: None,
            result: None,
            evidence_refs: None,
        }
    }

    fn make_checkpoint(id: &str, run_id: &str) -> CheckpointRow {
        CheckpointRow {
            id: id.into(),
            run_id: run_id.into(),
            step_id: None,
            created_at: Utc::now().to_rfc3339(),
            state_snapshot: b"snapshot data".to_vec(),
            rollback_commands: None,
        }
    }

    fn make_approval(id: &str, run_id: &str) -> ApprovalRow {
        let now = Utc::now().to_rfc3339();
        ApprovalRow {
            id: id.into(),
            run_id: run_id.into(),
            action: "delete_file".into(),
            target: Some("/tmp/test".into()),
            risk_score: 0.8,
            created_at: now.clone(),
            expires_at: now,
            status: ApprovalStatus::Pending,
        }
    }

    // --- DB Init ---

    #[test]
    fn test_in_memory() {
        let db = HydraDb::in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_init_file_db() {
        let dir = std::env::temp_dir().join(format!("hydra_db_test_{}", std::process::id()));
        let path = dir.join("test.db");
        let db = HydraDb::init(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate() {
        let db = HydraDb::in_memory().unwrap();
        db.migrate().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_clone_shares_connection() {
        let db = HydraDb::in_memory().unwrap();
        let db2 = db.clone();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        let run = db2.get_run("r1").unwrap();
        assert_eq!(run.intent, "test");
    }

    // --- RunStatus ---

    #[test]
    fn test_run_status_as_str() {
        assert_eq!(RunStatus::Pending.as_str(), "pending");
        assert_eq!(RunStatus::Running.as_str(), "running");
        assert_eq!(RunStatus::Paused.as_str(), "paused");
        assert_eq!(RunStatus::Completed.as_str(), "completed");
        assert_eq!(RunStatus::Failed.as_str(), "failed");
        assert_eq!(RunStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_run_status_from_str() {
        assert_eq!(RunStatus::from_str("pending"), Some(RunStatus::Pending));
        assert_eq!(RunStatus::from_str("running"), Some(RunStatus::Running));
        assert_eq!(RunStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_run_status_serde() {
        for s in [RunStatus::Pending, RunStatus::Running, RunStatus::Paused, RunStatus::Completed, RunStatus::Failed, RunStatus::Cancelled] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: RunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- StepStatus ---

    #[test]
    fn test_step_status_as_str() {
        assert_eq!(StepStatus::Pending.as_str(), "pending");
        assert_eq!(StepStatus::Running.as_str(), "running");
        assert_eq!(StepStatus::Completed.as_str(), "completed");
        assert_eq!(StepStatus::Failed.as_str(), "failed");
        assert_eq!(StepStatus::Skipped.as_str(), "skipped");
    }

    #[test]
    fn test_step_status_from_str() {
        assert_eq!(StepStatus::from_str("skipped"), Some(StepStatus::Skipped));
        assert_eq!(StepStatus::from_str("nope"), None);
    }

    #[test]
    fn test_step_status_serde() {
        for s in [StepStatus::Pending, StepStatus::Running, StepStatus::Completed, StepStatus::Failed, StepStatus::Skipped] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: StepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- ApprovalStatus ---

    #[test]
    fn test_approval_status_as_str() {
        assert_eq!(ApprovalStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalStatus::Denied.as_str(), "denied");
        assert_eq!(ApprovalStatus::Expired.as_str(), "expired");
    }

    #[test]
    fn test_approval_status_from_str() {
        assert_eq!(ApprovalStatus::from_str("approved"), Some(ApprovalStatus::Approved));
        assert_eq!(ApprovalStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_approval_status_serde() {
        for s in [ApprovalStatus::Pending, ApprovalStatus::Approved, ApprovalStatus::Denied, ApprovalStatus::Expired] {
            let json = serde_json::to_string(&s).unwrap();
            let restored: ApprovalStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }

    // --- CRUD Runs ---

    #[test]
    fn test_create_and_get_run() {
        let db = HydraDb::in_memory().unwrap();
        let run = make_run("r1", "refactor code", RunStatus::Pending);
        db.create_run(&run).unwrap();
        let fetched = db.get_run("r1").unwrap();
        assert_eq!(fetched.intent, "refactor code");
        assert_eq!(fetched.status, RunStatus::Pending);
    }

    #[test]
    fn test_get_run_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.get_run("nonexistent").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_update_run_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        db.update_run_status("r1", RunStatus::Running, None).unwrap();
        let run = db.get_run("r1").unwrap();
        assert_eq!(run.status, RunStatus::Running);
    }

    #[test]
    fn test_update_run_status_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_run_status("nope", RunStatus::Running, None).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_runs_all() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "a", RunStatus::Pending)).unwrap();
        db.create_run(&make_run("r2", "b", RunStatus::Running)).unwrap();
        let runs = db.list_runs(None).unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn test_list_runs_by_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "a", RunStatus::Pending)).unwrap();
        db.create_run(&make_run("r2", "b", RunStatus::Running)).unwrap();
        let pending = db.list_runs(Some(RunStatus::Pending)).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "r1");
    }

    #[test]
    fn test_list_runs_empty() {
        let db = HydraDb::in_memory().unwrap();
        let runs = db.list_runs(None).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn test_delete_run() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Pending)).unwrap();
        db.delete_run("r1").unwrap();
        assert!(matches!(db.get_run("r1").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_delete_run_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.delete_run("nope").unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- CRUD Steps ---

    #[test]
    fn test_create_and_get_step() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let step = make_step("s1", "r1", 1);
        db.create_step(&step).unwrap();
        let fetched = db.get_step("s1").unwrap();
        assert_eq!(fetched.run_id, "r1");
        assert_eq!(fetched.sequence, 1);
    }

    #[test]
    fn test_get_step_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_step("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_list_steps() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.create_step(&make_step("s2", "r1", 2)).unwrap();
        let steps = db.list_steps("r1").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].sequence, 1);
        assert_eq!(steps[1].sequence, 2);
    }

    #[test]
    fn test_update_step_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.update_step_status("s1", StepStatus::Completed, Some("2026-01-01"), Some("ok")).unwrap();
        let step = db.get_step("s1").unwrap();
        assert_eq!(step.status, StepStatus::Completed);
        assert_eq!(step.result, Some("ok".into()));
    }

    #[test]
    fn test_update_step_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_step_status("nope", StepStatus::Failed, None, None).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    // --- CRUD Checkpoints ---

    #[test]
    fn test_create_and_get_checkpoint() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let cp = make_checkpoint("cp1", "r1");
        db.create_checkpoint(&cp).unwrap();
        let fetched = db.get_checkpoint("cp1").unwrap();
        assert_eq!(fetched.state_snapshot, b"snapshot data");
    }

    #[test]
    fn test_get_checkpoint_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_checkpoint("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_list_checkpoints() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_checkpoint(&make_checkpoint("cp1", "r1")).unwrap();
        db.create_checkpoint(&make_checkpoint("cp2", "r1")).unwrap();
        let cps = db.list_checkpoints("r1").unwrap();
        assert_eq!(cps.len(), 2);
    }

    // --- CRUD Approvals ---

    #[test]
    fn test_create_and_get_approval() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        let a = make_approval("a1", "r1");
        db.create_approval(&a).unwrap();
        let fetched = db.get_approval("a1").unwrap();
        assert_eq!(fetched.action, "delete_file");
        assert_eq!(fetched.risk_score, 0.8);
    }

    #[test]
    fn test_get_approval_not_found() {
        let db = HydraDb::in_memory().unwrap();
        assert!(matches!(db.get_approval("nope").unwrap_err(), DbError::NotFound(_)));
    }

    #[test]
    fn test_update_approval_status() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_approval(&make_approval("a1", "r1")).unwrap();
        db.update_approval_status("a1", ApprovalStatus::Approved).unwrap();
        let a = db.get_approval("a1").unwrap();
        assert_eq!(a.status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_update_approval_not_found() {
        let db = HydraDb::in_memory().unwrap();
        let err = db.update_approval_status("nope", ApprovalStatus::Denied).unwrap_err();
        assert!(matches!(err, DbError::NotFound(_)));
    }

    #[test]
    fn test_list_pending_approvals() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_approval(&make_approval("a1", "r1")).unwrap();
        db.create_approval(&make_approval("a2", "r1")).unwrap();
        db.update_approval_status("a2", ApprovalStatus::Approved).unwrap();
        let pending = db.list_pending_approvals().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "a1");
    }

    // --- Row types ---

    #[test]
    fn test_run_row_serde() {
        let run = make_run("r1", "test", RunStatus::Completed);
        let json = serde_json::to_string(&run).unwrap();
        let restored: RunRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "r1");
        assert_eq!(restored.status, RunStatus::Completed);
    }

    #[test]
    fn test_step_row_serde() {
        let step = make_step("s1", "r1", 3);
        let json = serde_json::to_string(&step).unwrap();
        let restored: StepRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sequence, 3);
    }

    #[test]
    fn test_checkpoint_row_serde() {
        let cp = make_checkpoint("cp1", "r1");
        let json = serde_json::to_string(&cp).unwrap();
        let restored: CheckpointRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "cp1");
    }

    #[test]
    fn test_approval_row_serde() {
        let a = make_approval("a1", "r1");
        let json = serde_json::to_string(&a).unwrap();
        let restored: ApprovalRow = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.action, "delete_file");
    }

    // --- Run with metadata and parent ---

    #[test]
    fn test_run_with_metadata() {
        let db = HydraDb::in_memory().unwrap();
        let mut run = make_run("r1", "test", RunStatus::Pending);
        run.metadata = Some(r#"{"key":"value"}"#.into());
        db.create_run(&run).unwrap();
        let fetched = db.get_run("r1").unwrap();
        assert_eq!(fetched.metadata, Some(r#"{"key":"value"}"#.into()));
    }

    #[test]
    fn test_run_with_parent() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("parent", "parent intent", RunStatus::Running)).unwrap();
        let mut child = make_run("child", "child intent", RunStatus::Pending);
        child.parent_run_id = Some("parent".into());
        db.create_run(&child).unwrap();
        let fetched = db.get_run("child").unwrap();
        assert_eq!(fetched.parent_run_id, Some("parent".into()));
    }

    // --- Cascade delete ---

    #[test]
    fn test_delete_run_cascades_steps() {
        let db = HydraDb::in_memory().unwrap();
        db.create_run(&make_run("r1", "test", RunStatus::Running)).unwrap();
        db.create_step(&make_step("s1", "r1", 1)).unwrap();
        db.delete_run("r1").unwrap();
        assert!(matches!(db.get_step("s1").unwrap_err(), DbError::NotFound(_)));
    }

    // --- Connection sharing ---

    #[test]
    fn test_connection_returns_arc() {
        let db = HydraDb::in_memory().unwrap();
        let conn = db.connection();
        let guard = conn.lock();
        let v: u32 = guard.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| row.get(0)).unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }
}
