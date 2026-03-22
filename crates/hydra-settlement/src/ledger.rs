//! SettlementLedger — append-only store of all settlement records.
//! Constitutional: immutable once written. Queryable. Auditable.

use crate::{
    constants::MAX_SETTLEMENT_RECORDS, errors::SettlementError, period::SettlementPeriod,
    record::SettlementRecord,
};

/// Query parameters for the settlement ledger.
#[derive(Debug, Clone, Default)]
pub struct SettlementQuery {
    pub domain: Option<String>,
    pub action_id: Option<String>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub success_only: bool,
    pub limit: Option<usize>,
}

/// The settlement ledger — append-only, never deletable.
#[derive(Default)]
pub struct SettlementLedger {
    records: Vec<SettlementRecord>,
    db: Option<crate::persistence::SettlementDb>,
}


impl std::fmt::Debug for SettlementLedger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettlementLedger")
            .field("records", &self.records.len())
            .field("has_db", &self.db.is_some())
            .finish()
    }
}

impl SettlementLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a ledger backed by SQLite persistence, loading existing records.
    pub fn open() -> Self {
        match crate::persistence::SettlementDb::open() {
            Ok(db) => {
                let records = db.load_all();
                eprintln!("hydra: settlement loaded {} records from db", records.len());
                Self {
                    records,
                    db: Some(db),
                }
            }
            Err(e) => {
                eprintln!("hydra: settlement db open failed: {}, using in-memory", e);
                Self::new()
            }
        }
    }

    /// Append a settlement record. Immutable after this.
    pub fn settle(&mut self, record: SettlementRecord) -> Result<(), SettlementError> {
        if self.records.len() >= MAX_SETTLEMENT_RECORDS {
            return Err(SettlementError::LedgerFull {
                max: MAX_SETTLEMENT_RECORDS,
            });
        }
        if let Some(ref db) = self.db {
            db.insert(&record);
        }
        self.records.push(record);
        Ok(())
    }

    /// Query records by filters.
    pub fn query(&self, q: &SettlementQuery) -> Vec<&SettlementRecord> {
        let limit = q.limit.unwrap_or(crate::constants::MAX_PERIOD_RECORDS);
        let mut results: Vec<&SettlementRecord> = self
            .records
            .iter()
            .filter(|r| {
                if let Some(d) = &q.domain {
                    if &r.domain != d {
                        return false;
                    }
                }
                if let Some(a) = &q.action_id {
                    if &r.action_id != a {
                        return false;
                    }
                }
                if let Some(since) = &q.since {
                    if r.settled_at < *since {
                        return false;
                    }
                }
                if let Some(until) = &q.until {
                    if r.settled_at > *until {
                        return false;
                    }
                }
                if q.success_only && !r.outcome.is_success() {
                    return false;
                }
                true
            })
            .collect();

        // Most recent first
        results.sort_by(|a, b| b.settled_at.cmp(&a.settled_at));
        results.truncate(limit);
        results
    }

    /// Build a settlement period from records in a time window.
    pub fn period(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> SettlementPeriod {
        let records: Vec<&SettlementRecord> = self
            .records
            .iter()
            .filter(|r| r.settled_at >= start && r.settled_at <= end)
            .collect();
        SettlementPeriod::from_records(start, end, &records)
    }

    pub fn count(&self) -> usize {
        self.records.len()
    }

    pub fn get_by_task(&self, task_id: &str) -> Option<&SettlementRecord> {
        self.records.iter().find(|r| r.task_id == task_id)
    }

    /// Total lifetime cost across all records.
    pub fn lifetime_cost(&self) -> f64 {
        self.records.iter().map(|r| r.total_cost).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostClass, CostItem};
    use crate::record::{Outcome, SettlementRecord};

    fn make(task_id: &str, domain: &str) -> SettlementRecord {
        SettlementRecord::new(
            task_id,
            "a.id",
            domain,
            "intent",
            Outcome::Success {
                description: "done".into(),
            },
            vec![CostItem::new(CostClass::DirectExecution, 1000, 5.0, 1000)],
            1000,
            1,
        )
    }

    #[test]
    fn settle_and_query() {
        let mut ledger = SettlementLedger::new();
        ledger.settle(make("t1", "engineering")).expect("settle t1");
        ledger.settle(make("t2", "finance")).expect("settle t2");
        assert_eq!(ledger.count(), 2);

        let q = SettlementQuery {
            domain: Some("engineering".into()),
            ..Default::default()
        };
        let results = ledger.query(&q);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn append_only_no_deletion() {
        let mut ledger = SettlementLedger::new();
        ledger.settle(make("t1", "test")).expect("settle t1");
        let count = ledger.count();
        // No remove method exists — append only
        ledger.settle(make("t2", "test")).expect("settle t2");
        assert_eq!(ledger.count(), count + 1);
    }

    #[test]
    fn period_aggregates_window() {
        let mut ledger = SettlementLedger::new();
        for i in 0..5 {
            ledger
                .settle(make(&format!("t{}", i), "engineering"))
                .expect("settle");
        }
        let now = chrono::Utc::now();
        let start = now - chrono::Duration::hours(1);
        let p = ledger.period(start, now);
        assert_eq!(p.record_count, 5);
    }
}
