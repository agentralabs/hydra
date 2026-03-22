//! SettlementPeriod — aggregated settlement for a time window.
//! The basis for operational intelligence reports.

use crate::record::SettlementRecord;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregated settlement statistics for one time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementPeriod {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub record_count: usize,
    pub success_count: usize,
    pub denied_count: usize,
    pub total_cost: f64,
    pub overhead_cost: f64,
    pub avg_cost: f64,
    pub avg_duration_ms: f64,
    pub avg_attempts: f64,
    /// Cost breakdown by domain (e.g. "engineering": 45.2).
    pub cost_by_domain: HashMap<String, f64>,
    /// Cost breakdown by class (e.g. "direct": 30.1, "sister:AgenticMemory": 10.2).
    pub cost_by_class: HashMap<String, f64>,
    /// Efficiency: successful outcomes / total cost.
    pub efficiency: f64,
    /// Is spend trending up this period vs prior? None if no prior data.
    pub spend_trend: Option<SpendTrend>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpendTrend {
    Increasing { pct_change: f64 },
    Decreasing { pct_change: f64 },
    Stable,
}

impl SettlementPeriod {
    /// Build a period from a set of settlement records.
    pub fn from_records(
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
        records: &[&SettlementRecord],
    ) -> Self {
        let n = records.len();
        if n == 0 {
            return Self::empty(start, end);
        }

        let success_count = records.iter().filter(|r| r.outcome.is_success()).count();
        let denied_count = n - success_count;
        let total_cost: f64 = records.iter().map(|r| r.total_cost).sum();
        let overhead_cost: f64 = records.iter().map(|r| r.overhead_cost()).sum();
        let avg_cost = total_cost / n as f64;
        let avg_dur = records.iter().map(|r| r.duration_ms as f64).sum::<f64>() / n as f64;
        let avg_attempts = records.iter().map(|r| r.attempt_count as f64).sum::<f64>() / n as f64;
        let efficiency = if total_cost > 1e-10 {
            success_count as f64 / total_cost
        } else {
            0.0
        };

        // Aggregate cost by domain
        let mut cost_by_domain: HashMap<String, f64> = HashMap::new();
        for r in records {
            *cost_by_domain.entry(r.domain.clone()).or_insert(0.0) += r.total_cost;
        }

        // Aggregate cost by class
        let mut cost_by_class: HashMap<String, f64> = HashMap::new();
        for r in records {
            for (class, cost) in r.cost_by_class() {
                *cost_by_class.entry(class).or_insert(0.0) += cost;
            }
        }

        Self {
            start,
            end,
            record_count: n,
            success_count,
            denied_count,
            total_cost,
            overhead_cost,
            avg_cost,
            avg_duration_ms: avg_dur,
            avg_attempts,
            cost_by_domain,
            cost_by_class,
            efficiency,
            spend_trend: None,
        }
    }

    fn empty(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            start,
            end,
            record_count: 0,
            success_count: 0,
            denied_count: 0,
            total_cost: 0.0,
            overhead_cost: 0.0,
            avg_cost: 0.0,
            avg_duration_ms: 0.0,
            avg_attempts: 0.0,
            cost_by_domain: HashMap::new(),
            cost_by_class: HashMap::new(),
            efficiency: 0.0,
            spend_trend: None,
        }
    }

    pub fn with_trend(mut self, prior: &SettlementPeriod) -> Self {
        if prior.total_cost < 1e-10 {
            self.spend_trend = Some(SpendTrend::Stable);
            return self;
        }
        let pct = (self.total_cost - prior.total_cost) / prior.total_cost * 100.0;
        self.spend_trend = Some(if pct > 5.0 {
            SpendTrend::Increasing { pct_change: pct }
        } else if pct < -5.0 {
            SpendTrend::Decreasing {
                pct_change: pct.abs(),
            }
        } else {
            SpendTrend::Stable
        });
        self
    }

    /// Top spending domain in this period.
    pub fn top_domain(&self) -> Option<(&String, f64)> {
        self.cost_by_domain
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k, *v))
    }

    /// Human-readable brief for TUI / reports.
    pub fn brief(&self) -> String {
        format!(
            "{} tasks | {:.1} total cost | {:.0}% success | efficiency={:.3}",
            self.record_count,
            self.total_cost,
            (self.success_count as f64 / self.record_count.max(1) as f64) * 100.0,
            self.efficiency,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostClass, CostItem};
    use crate::record::{Outcome, SettlementRecord};

    fn make_record(domain: &str, success: bool, cost_tokens: u64) -> SettlementRecord {
        SettlementRecord::new(
            "t1",
            "a.id",
            domain,
            "intent",
            if success {
                Outcome::Success {
                    description: "done".into(),
                }
            } else {
                Outcome::HardDenied {
                    evidence: "denied".into(),
                }
            },
            vec![CostItem::new(
                CostClass::DirectExecution,
                cost_tokens,
                5.0,
                1000,
            )],
            1000,
            1,
        )
    }

    #[test]
    fn period_aggregates_correctly() {
        let r1 = make_record("engineering", true, 2000);
        let r2 = make_record("finance", true, 1000);
        let r3 = make_record("engineering", false, 500);

        let now = chrono::Utc::now();
        let p =
            SettlementPeriod::from_records(now - chrono::Duration::days(1), now, &[&r1, &r2, &r3]);
        assert_eq!(p.record_count, 3);
        assert_eq!(p.success_count, 2);
        assert_eq!(p.denied_count, 1);
        assert!(p.total_cost > 0.0);
        assert_eq!(p.cost_by_domain.len(), 2);
    }

    #[test]
    fn period_top_domain() {
        let r1 = make_record("engineering", true, 5000);
        let r2 = make_record("finance", true, 1000);
        let now = chrono::Utc::now();
        let p = SettlementPeriod::from_records(now - chrono::Duration::days(1), now, &[&r1, &r2]);
        let (top, _) = p.top_domain().expect("should have top domain");
        assert_eq!(top, "engineering");
    }

    #[test]
    fn trend_computed_from_prior() {
        let r1 = make_record("test", true, 1000);
        let r2 = make_record("test", true, 2000);
        let now = chrono::Utc::now();
        let prior = SettlementPeriod::from_records(
            now - chrono::Duration::days(2),
            now - chrono::Duration::days(1),
            &[&r1],
        );
        let current = SettlementPeriod::from_records(now - chrono::Duration::days(1), now, &[&r2])
            .with_trend(&prior);
        assert!(matches!(
            current.spend_trend,
            Some(SpendTrend::Increasing { .. })
        ));
    }
}
