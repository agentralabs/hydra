use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::{params, Connection};

use crate::store_types::*;

/// Query/read methods for HydraDb — core entities (runs, steps, checkpoints, approvals, receipts)
impl crate::store::HydraDb {
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

    // ═══════════════════════════════════════════════════════
    // STEPS
    // ═══════════════════════════════════════════════════════

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

    // ═══════════════════════════════════════════════════════
    // CHECKPOINTS
    // ═══════════════════════════════════════════════════════

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

    /// Expose the shared connection for subsystems (e.g. MessageStore)
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    // ═══════════════════════════════════════════════════════
    // RECEIPTS
    // ═══════════════════════════════════════════════════════

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
}
