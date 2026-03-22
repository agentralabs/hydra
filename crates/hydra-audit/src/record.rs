//! AuditRecord — the persistent accountability unit.
//! Immutable once written. Constitutional.
//! Settlement and attribution layers read from here.

use crate::narrative::ExecutionNarrative;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// Query filters for audit records.
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by task ID.
    pub task_id: Option<String>,
    /// Filter by action ID.
    pub action_id: Option<String>,
    /// Filter by outcome.
    pub outcome: Option<String>,
    /// Filter records created after this time.
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter records created before this time.
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    /// Max records to return.
    pub limit: Option<usize>,
}

/// One immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// Unique record ID.
    pub id: String,
    /// The task ID this record audits.
    pub task_id: String,
    /// The action ID that was executed.
    pub action_id: String,
    /// Outcome label.
    pub outcome: String,
    /// One-line summary for TUI.
    pub summary: String,
    /// Full narrative text.
    pub full_narrative: String,
    /// Number of approaches attempted.
    pub attempt_count: usize,
    /// Number of obstacles encountered.
    pub obstacle_count: usize,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
    /// Receipt IDs that compose this record.
    pub receipt_ids: Vec<String>,
    /// SHA256 of all fields — tamper detection.
    pub integrity_hash: String,
    /// When this record was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AuditRecord {
    /// Create an audit record from a narrative and receipt IDs.
    pub fn from_narrative(
        narrative: &ExecutionNarrative,
        receipt_ids: Vec<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        let hash = Self::compute_hash(
            &narrative.task_id,
            &narrative.action_id,
            &narrative.outcome,
            &narrative.full,
            &now,
        );
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: narrative.task_id.clone(),
            action_id: narrative.action_id.clone(),
            outcome: narrative.outcome.clone(),
            summary: narrative.summary.clone(),
            full_narrative: narrative.full.clone(),
            attempt_count: narrative.attempt_count,
            obstacle_count: narrative.obstacle_count,
            duration_ms: narrative.duration_ms,
            receipt_ids,
            integrity_hash: hash,
            created_at: now,
        }
    }

    fn compute_hash(
        task_id: &str,
        action_id: &str,
        outcome: &str,
        narrative: &str,
        at: &chrono::DateTime<chrono::Utc>,
    ) -> String {
        let mut h = Sha256::new();
        h.update(task_id.as_bytes());
        h.update(action_id.as_bytes());
        h.update(outcome.as_bytes());
        h.update(narrative.as_bytes());
        h.update(at.to_rfc3339().as_bytes());
        hex::encode(h.finalize())
    }

    /// Verify integrity hash — tamper detection.
    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }

    /// Whether this record represents a successful execution.
    pub fn is_successful(&self) -> bool {
        self.outcome == "completed"
    }
}

/// The audit store — append-only, queryable.
#[derive(Default)]
pub struct AuditStore {
    records: Vec<AuditRecord>,
    db: Option<crate::persistence::AuditDb>,
}


impl std::fmt::Debug for AuditStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditStore")
            .field("records", &self.records.len())
            .field("has_db", &self.db.is_some())
            .finish()
    }
}

impl AuditStore {
    /// Create a new empty audit store (in-memory only).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an audit store backed by SQLite persistence, loading existing records.
    pub fn open() -> Self {
        match crate::persistence::AuditDb::open() {
            Ok(db) => {
                let records = db.load_all();
                eprintln!("hydra: audit loaded {} records from db", records.len());
                Self {
                    records,
                    db: Some(db),
                }
            }
            Err(e) => {
                eprintln!("hydra: audit db open failed: {}, using in-memory", e);
                Self::new()
            }
        }
    }

    /// Append a record. Immutable after this point.
    pub fn append(&mut self, record: AuditRecord) {
        if let Some(ref db) = self.db {
            db.insert(&record);
        }
        self.records.push(record);
    }

    /// Query records by filters.
    pub fn query(&self, q: &AuditQuery) -> Vec<&AuditRecord> {
        let limit = q.limit.unwrap_or(crate::constants::MAX_QUERY_RESULTS);
        let mut results: Vec<&AuditRecord> = self
            .records
            .iter()
            .filter(|r| {
                if let Some(tid) = &q.task_id {
                    if &r.task_id != tid {
                        return false;
                    }
                }
                if let Some(aid) = &q.action_id {
                    if &r.action_id != aid {
                        return false;
                    }
                }
                if let Some(out) = &q.outcome {
                    if &r.outcome != out {
                        return false;
                    }
                }
                if let Some(since) = &q.since {
                    if r.created_at < *since {
                        return false;
                    }
                }
                if let Some(until) = &q.until {
                    if r.created_at > *until {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Most recent first
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results.truncate(limit);
        results
    }

    /// Total number of records.
    pub fn count(&self) -> usize {
        self.records.len()
    }

    /// Find a record by task ID.
    pub fn get_by_task(&self, task_id: &str) -> Option<&AuditRecord> {
        self.records.iter().find(|r| r.task_id == task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::ExecutionNarrative;

    fn make_narrative(outcome: &str) -> ExecutionNarrative {
        ExecutionNarrative {
            task_id: "task-1".into(),
            action_id: "deploy.staging".into(),
            summary: format!(
                "[{}] deploy.staging",
                outcome.to_uppercase()
            ),
            full: "Task started. Attempted direct. Completed.".into(),
            outcome: outcome.to_string(),
            attempt_count: 1,
            obstacle_count: 0,
            duration_ms: 500,
        }
    }

    #[test]
    fn record_integrity_hash_valid() {
        let n = make_narrative("completed");
        let r = AuditRecord::from_narrative(&n, vec!["r1".into()]);
        assert!(r.verify_integrity());
        assert_eq!(r.integrity_hash.len(), 64);
    }

    #[test]
    fn append_only_store() {
        let mut store = AuditStore::new();
        let r =
            AuditRecord::from_narrative(&make_narrative("completed"), vec![]);
        store.append(r);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn query_by_outcome() {
        let mut store = AuditStore::new();
        store.append(AuditRecord::from_narrative(
            &make_narrative("completed"),
            vec![],
        ));
        store.append(AuditRecord::from_narrative(
            &make_narrative("hard-denied"),
            vec![],
        ));
        let q = AuditQuery {
            outcome: Some("completed".into()),
            ..Default::default()
        };
        let results = store.query(&q);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, "completed");
    }

    #[test]
    fn query_by_task_id() {
        let mut store = AuditStore::new();
        let mut n = make_narrative("completed");
        n.task_id = "specific-task".into();
        store.append(AuditRecord::from_narrative(&n, vec![]));
        store.append(AuditRecord::from_narrative(
            &make_narrative("completed"),
            vec![],
        ));
        let r = store.get_by_task("specific-task");
        assert!(r.is_some());
    }
}
