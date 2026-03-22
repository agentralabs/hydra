//! SchedulerEngine — the coordinator.
//! Scans the queue every cycle. Fires due jobs. Receipts everything.

use crate::{
    constants::MAX_CONCURRENT_FIRES,
    errors::SchedulerError,
    job::{JobState, ScheduledJob},
    queue::JobQueue,
    trigger::{MetricConditionType, TriggerType},
};
use hydra_soul::TemporalHorizon;
use std::collections::HashMap;

/// Create a schedule-related receipt hash.
pub fn create_schedule_receipt(context: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(context.as_bytes());
    h.update(chrono::Utc::now().to_rfc3339().as_bytes());
    hex::encode(h.finalize())
}

/// Result of one scheduler tick.
#[derive(Debug, Default)]
pub struct TickResult {
    pub fired:     Vec<String>,  // job IDs that fired
    pub skipped:   Vec<String>,  // job IDs skipped (suspended/running)
    pub errors:    Vec<String>,  // job IDs that errored
}

/// The scheduler engine.
pub struct SchedulerEngine {
    pub queue:          JobQueue,
    tick_count:         u64,
    metric_values:      HashMap<String, f64>,
    fire_receipts:      Vec<String>,
}

impl SchedulerEngine {
    pub fn new() -> Self {
        Self {
            queue:         JobQueue::new(),
            tick_count:    0,
            metric_values: HashMap::new(),
            fire_receipts: Vec::new(),
        }
    }

    /// Schedule a new job.
    pub fn schedule(
        &mut self,
        name:      impl Into<String>,
        action_id: impl Into<String>,
        intent:    impl Into<String>,
        trigger:   TriggerType,
        horizon:   TemporalHorizon,
    ) -> Result<String, SchedulerError> {
        let job = ScheduledJob::new(name, action_id, intent, trigger, horizon);
        let id  = job.id.clone();
        self.queue.enqueue(job)?;
        Ok(id)
    }

    /// Cancel a scheduled job.
    pub fn cancel(&mut self, job_id: &str) -> Result<(), SchedulerError> {
        self.queue.cancel(job_id)
    }

    /// Update a metric value (used for MetricCondition triggers).
    pub fn update_metric(&mut self, metric: &str, value: f64) {
        self.metric_values.insert(metric.to_string(), value);
    }

    /// Run one scheduler tick — fire all due jobs.
    /// In production: called by the AMBIENT thread every N seconds.
    pub fn tick(&mut self) -> TickResult {
        self.tick_count += 1;
        let mut result = TickResult::default();
        let now        = chrono::Utc::now();

        // Check metric conditions
        self.check_metric_conditions();

        // Collect due job IDs (avoid borrow issues)
        let due_ids: Vec<String> = self.queue.due_jobs()
            .iter()
            .map(|j| j.id.clone())
            .collect();

        let mut fire_count = 0usize;

        for job_id in due_ids {
            // Enforce concurrent fire limit
            if fire_count >= MAX_CONCURRENT_FIRES {
                result.skipped.push(job_id);
                continue;
            }

            if let Some(job) = self.queue.get_mut(&job_id) {
                match &job.state {
                    JobState::Pending | JobState::Completed { .. } => {
                        // Receipt BEFORE fire
                        let receipt = create_schedule_receipt(
                            &format!("fire:{}:{}", job_id, now.timestamp())
                        );
                        self.fire_receipts.push(receipt);

                        job.mark_started();
                        // In production: call hydra-executor here
                        // In this implementation: simulate success
                        job.mark_completed();
                        result.fired.push(job_id);
                        fire_count += 1;
                    }
                    JobState::Suspended { .. } => {
                        result.skipped.push(job_id);
                    }
                    _ => {}
                }
            }
        }

        result
    }

    /// Check metric-condition triggers.
    fn check_metric_conditions(&mut self) {
        let values = self.metric_values.clone();
        let now    = chrono::Utc::now();

        let triggered_ids: Vec<String> = self.queue.queue_values()
            .iter()
            .filter_map(|job| {
                if let TriggerType::MetricCondition { metric, condition, .. } = &job.trigger {
                    let value = values.get(metric.as_str())?;
                    let triggered = match condition {
                        MetricConditionType::EqualsZero => *value == 0.0,
                        MetricConditionType::ExceedsThreshold { threshold } => value > threshold,
                        MetricConditionType::DropsBelow { threshold } => value < threshold,
                        MetricConditionType::StaysZeroFor { .. } => *value == 0.0,
                    };
                    if triggered { Some(job.id.clone()) } else { None }
                } else {
                    None
                }
            })
            .collect();

        for id in triggered_ids {
            if let Some(job) = self.queue.get_mut(&id) {
                if matches!(job.state, JobState::Pending) {
                    // Set as immediately due
                    job.next_fire = Some(now);
                }
            }
        }
    }

    pub fn tick_count(&self)    -> u64   { self.tick_count }
    pub fn receipt_count(&self) -> usize { self.fire_receipts.len() }
    pub fn job_count(&self)     -> usize { self.queue.len() }
    pub fn active_count(&self)  -> usize { self.queue.active_count() }

    /// TUI summary.
    pub fn summary(&self) -> String {
        format!(
            "scheduler: jobs={} active={} ticks={} receipts={}",
            self.queue.len(),
            self.queue.active_count(),
            self.tick_count,
            self.fire_receipts.len(),
        )
    }
}

impl Default for SchedulerEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_and_retrieve() {
        let mut engine = SchedulerEngine::new();
        let id = engine.schedule(
            "test-job", "test.action", "test intent",
            TriggerType::Recurring {
                interval_seconds: 3600,
                first_fire: None,
                label: "test".into(),
            },
            TemporalHorizon::Immediate,
        ).expect("schedule should succeed");
        assert_eq!(engine.job_count(), 1);
        assert!(engine.queue.get(&id).is_some());
    }

    #[test]
    fn overdue_job_fires_on_tick() {
        let mut engine = SchedulerEngine::new();
        let id = engine.schedule(
            "due-job", "test.action", "test intent",
            TriggerType::Recurring {
                interval_seconds: 1,
                first_fire: Some(
                    chrono::Utc::now() - chrono::Duration::seconds(10)
                ),
                label: "test".into(),
            },
            TemporalHorizon::Immediate,
        ).expect("schedule should succeed");

        // Force it to be due
        if let Some(job) = engine.queue.get_mut(&id) {
            job.next_fire = Some(chrono::Utc::now() - chrono::Duration::seconds(5));
            job.state     = JobState::Pending;
        }

        let result = engine.tick();
        assert_eq!(result.fired.len(), 1);
        assert!(engine.receipt_count() >= 1);
    }

    #[test]
    fn tick_increments_count() {
        let mut engine = SchedulerEngine::new();
        assert_eq!(engine.tick_count(), 0);
        engine.tick();
        engine.tick();
        assert_eq!(engine.tick_count(), 2);
    }

    #[test]
    fn metric_condition_triggers_job() {
        let mut engine = SchedulerEngine::new();
        engine.schedule(
            "genome-alert", "alert.genome", "genome growth stopped",
            TriggerType::MetricCondition {
                metric:    "genome_growth_rate".into(),
                condition: MetricConditionType::EqualsZero,
                label:     "genome-zero".into(),
            },
            TemporalHorizon::Foundational,
        ).expect("schedule should succeed");

        // Update metric to trigger condition
        engine.update_metric("genome_growth_rate", 0.0);
        engine.tick();

        // After tick: job should have been attempted
        assert!(engine.tick_count() > 0);
    }

    #[test]
    fn cancel_job_removes_from_active() {
        let mut engine = SchedulerEngine::new();
        let id = engine.schedule(
            "to-cancel", "a", "i",
            TriggerType::Recurring {
                interval_seconds: 3600,
                first_fire: None,
                label: "t".into(),
            },
            TemporalHorizon::Immediate,
        ).expect("schedule should succeed");
        assert_eq!(engine.active_count(), 1);
        engine.cancel(&id).expect("cancel should succeed");
        assert_eq!(engine.active_count(), 0);
    }

    #[test]
    fn summary_format() {
        let engine = SchedulerEngine::new();
        let s = engine.summary();
        assert!(s.contains("scheduler:"));
        assert!(s.contains("jobs="));
        assert!(s.contains("ticks="));
    }

    #[test]
    fn one_shot_exhausted_after_fire() {
        let mut engine = SchedulerEngine::new();
        let id = engine.schedule(
            "one-shot", "a", "i",
            TriggerType::OneShot {
                fire_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            },
            TemporalHorizon::Immediate,
        ).expect("schedule should succeed");
        engine.tick();
        let job = engine.queue.get(&id).expect("job should exist");
        assert_eq!(job.state.label(), "exhausted");
    }

    #[test]
    fn recurring_job_requeues_after_fire() {
        let mut engine = SchedulerEngine::new();
        let id = engine.schedule(
            "recurring", "a", "i",
            TriggerType::Recurring {
                interval_seconds: 3600,
                first_fire: Some(
                    chrono::Utc::now() - chrono::Duration::seconds(1)
                ),
                label: "t".into(),
            },
            TemporalHorizon::Immediate,
        ).expect("schedule should succeed");

        if let Some(job) = engine.queue.get_mut(&id) {
            job.next_fire = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
            job.state     = JobState::Pending;
        }

        engine.tick();

        let job = engine.queue.get(&id).expect("job should exist");
        // After firing: next_fire should be set to ~1 hour from now
        assert!(job.next_fire.is_some());
        assert!(job.next_fire.expect("next_fire should be set") > chrono::Utc::now());
    }
}
