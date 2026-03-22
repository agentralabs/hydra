//! ScheduledJob — one unit of scheduled work.
//! Every job is receipted when created and when it fires.

use crate::{
    constants::MAX_JOB_RETRIES,
    trigger::TriggerType,
};
use hydra_soul::TemporalHorizon;
use serde::{Deserialize, Serialize};

/// The state of a scheduled job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    /// Waiting to fire.
    Pending,
    /// Currently executing.
    Running { started_at: chrono::DateTime<chrono::Utc> },
    /// Completed successfully.
    Completed { last_run: chrono::DateTime<chrono::Utc>, run_count: u32 },
    /// One-shot job that has fired and is done.
    Exhausted,
    /// Suspended after too many failures.
    Suspended { reason: String, retry_after: chrono::DateTime<chrono::Utc> },
    /// Cancelled by the principal.
    Cancelled,
}

impl JobState {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Exhausted | Self::Cancelled)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending          => "pending",
            Self::Running { .. }   => "running",
            Self::Completed { .. } => "completed",
            Self::Exhausted        => "exhausted",
            Self::Suspended { .. } => "suspended",
            Self::Cancelled        => "cancelled",
        }
    }
}

/// One scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id:              String,
    /// Human-readable name.
    pub name:            String,
    /// The action to execute (action_id in hydra-executor registry).
    pub action_id:       String,
    /// The intent to pass to the executor.
    pub intent:          String,
    /// Trigger condition.
    pub trigger:         TriggerType,
    /// Current state.
    pub state:           JobState,
    /// Soul temporal horizon — affects care level.
    pub horizon:         TemporalHorizon,
    /// When does this job fire next?
    pub next_fire:       Option<chrono::DateTime<chrono::Utc>>,
    /// When did it last fire?
    pub last_fire:       Option<chrono::DateTime<chrono::Utc>>,
    /// How many times has it fired successfully?
    pub success_count:   u32,
    /// How many times has it failed?
    pub failure_count:   u32,
    /// Receipt ID for when this job was created.
    pub creation_receipt: String,
    pub created_at:      chrono::DateTime<chrono::Utc>,
    pub updated_at:      chrono::DateTime<chrono::Utc>,
}

impl ScheduledJob {
    pub fn new(
        name:      impl Into<String>,
        action_id: impl Into<String>,
        intent:    impl Into<String>,
        trigger:   TriggerType,
        horizon:   TemporalHorizon,
    ) -> Self {
        let now = chrono::Utc::now();

        // Receipt on creation — write-ahead
        let receipt = crate::engine::create_schedule_receipt(
            &format!("create:{}", uuid::Uuid::new_v4()),
        );

        let mut job = Self {
            id:               uuid::Uuid::new_v4().to_string(),
            name:             name.into(),
            action_id:        action_id.into(),
            intent:           intent.into(),
            trigger:          trigger.clone(),
            state:            JobState::Pending,
            horizon,
            next_fire:        None,
            last_fire:        None,
            success_count:    0,
            failure_count:    0,
            creation_receipt: receipt,
            created_at:       now,
            updated_at:       now,
        };

        // Compute first fire time
        // For one-shot triggers, always use fire_at directly (even if past —
        // the job should fire on the next tick).
        job.next_fire = match &trigger {
            TriggerType::OneShot { fire_at } => Some(*fire_at),
            _ => trigger.next_fire_after(&now, None),
        };
        job
    }

    pub fn is_due(&self, now: &chrono::DateTime<chrono::Utc>) -> bool {
        match &self.state {
            JobState::Pending | JobState::Completed { .. } => {
                self.next_fire.map(|f| f <= *now).unwrap_or(false)
            }
            _ => false,
        }
    }

    pub fn mark_started(&mut self) {
        self.state      = JobState::Running { started_at: chrono::Utc::now() };
        self.updated_at = chrono::Utc::now();
    }

    pub fn mark_completed(&mut self) {
        let now = chrono::Utc::now();
        self.success_count += 1;
        self.last_fire      = Some(now);
        self.updated_at     = now;

        // Recompute next fire for recurring jobs
        self.next_fire = self.trigger.next_fire_after(&now, self.last_fire.as_ref());

        self.state = match &self.trigger {
            TriggerType::OneShot { .. }               => JobState::Exhausted,
            TriggerType::ConstraintActivation { .. }  => JobState::Exhausted,
            _ => JobState::Completed { last_run: now, run_count: self.success_count },
        };
    }

    pub fn mark_failed(&mut self, reason: &str) {
        self.failure_count += 1;
        let now = chrono::Utc::now();
        self.updated_at = now;

        if self.failure_count >= MAX_JOB_RETRIES {
            self.state = JobState::Suspended {
                reason:      format!("Max retries ({}) exceeded: {}", MAX_JOB_RETRIES, reason),
                retry_after: now + chrono::Duration::seconds(
                    crate::constants::FAILED_JOB_RETRY_SECONDS as i64 * 4
                ),
            };
        } else {
            // Back to pending — will retry
            let retry_after = now + chrono::Duration::seconds(
                crate::constants::FAILED_JOB_RETRY_SECONDS as i64
            );
            self.next_fire = Some(retry_after);
            self.state = JobState::Pending;
        }
    }

    /// Care multiplier from temporal horizon — foundational jobs get more care.
    pub fn care_multiplier(&self) -> f64 {
        self.horizon.care_multiplier()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_job(interval_secs: u64) -> ScheduledJob {
        ScheduledJob::new(
            "test-job",
            "test.action",
            "test intent",
            TriggerType::Recurring {
                interval_seconds: interval_secs,
                first_fire: None,
                label: "test".into(),
            },
            TemporalHorizon::Immediate,
        )
    }

    #[test]
    fn new_job_is_pending() {
        let job = make_job(3600);
        assert_eq!(job.state.label(), "pending");
        assert!(job.next_fire.is_some());
    }

    #[test]
    fn job_not_due_immediately() {
        let job = make_job(3600);
        let now = chrono::Utc::now();
        // Just created — next fire is in 1 hour
        assert!(!job.is_due(&now));
    }

    #[test]
    fn job_due_when_past_next_fire() {
        let mut job = make_job(3600);
        // Set next fire in the past
        job.next_fire = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
        let now = chrono::Utc::now();
        assert!(job.is_due(&now));
    }

    #[test]
    fn mark_completed_increments_count() {
        let mut job = make_job(3600);
        job.mark_started();
        job.mark_completed();
        assert_eq!(job.success_count, 1);
        assert!(job.last_fire.is_some());
    }

    #[test]
    fn one_shot_exhausted_after_completion() {
        let mut job = ScheduledJob::new(
            "once", "a.id", "intent",
            TriggerType::OneShot {
                fire_at: chrono::Utc::now() - chrono::Duration::seconds(1),
            },
            TemporalHorizon::Immediate,
        );
        job.mark_started();
        job.mark_completed();
        assert_eq!(job.state.label(), "exhausted");
    }

    #[test]
    fn failed_job_retries_until_suspended() {
        let mut job = make_job(60);
        for _ in 0..MAX_JOB_RETRIES {
            job.mark_failed("test failure");
        }
        assert_eq!(job.state.label(), "suspended");
    }

    #[test]
    fn foundational_higher_care_than_immediate() {
        let immediate   = ScheduledJob::new("j", "a", "i",
            TriggerType::Recurring { interval_seconds: 60,
                first_fire: None, label: "t".into() },
            TemporalHorizon::Immediate);
        let foundational = ScheduledJob::new("j", "a", "i",
            TriggerType::Recurring { interval_seconds: 60,
                first_fire: None, label: "t".into() },
            TemporalHorizon::Foundational);
        assert!(foundational.care_multiplier() > immediate.care_multiplier());
    }
}
