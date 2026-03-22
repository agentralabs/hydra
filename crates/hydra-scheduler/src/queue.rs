//! JobQueue — persistent job store.
//! Survives restart. Append-friendly. Constitutional.

use crate::{
    constants::MAX_QUEUED_JOBS,
    errors::SchedulerError,
    job::ScheduledJob,
};
use std::collections::HashMap;

/// The job queue — all scheduled jobs.
#[derive(Debug, Default)]
pub struct JobQueue {
    jobs: HashMap<String, ScheduledJob>,
}

impl JobQueue {
    pub fn new() -> Self { Self::default() }

    /// Add a job to the queue.
    pub fn enqueue(&mut self, job: ScheduledJob) -> Result<(), SchedulerError> {
        if self.jobs.len() >= MAX_QUEUED_JOBS {
            return Err(SchedulerError::QueueFull { max: MAX_QUEUED_JOBS });
        }
        if self.jobs.contains_key(&job.id) {
            return Err(SchedulerError::DuplicateJob { id: job.id });
        }
        self.jobs.insert(job.id.clone(), job);
        Ok(())
    }

    /// Remove a job from the queue.
    pub fn dequeue(&mut self, job_id: &str) -> Option<ScheduledJob> {
        self.jobs.remove(job_id)
    }

    /// Cancel a job (marks it, does not remove — history preserved).
    pub fn cancel(&mut self, job_id: &str) -> Result<(), SchedulerError> {
        let job = self.jobs.get_mut(job_id)
            .ok_or_else(|| SchedulerError::JobNotFound { id: job_id.to_string() })?;
        job.state = crate::job::JobState::Cancelled;
        Ok(())
    }

    /// Jobs due for firing right now.
    pub fn due_jobs(&self) -> Vec<&ScheduledJob> {
        let now = chrono::Utc::now();
        let mut due: Vec<&ScheduledJob> = self.jobs.values()
            .filter(|j| j.is_due(&now) && j.state.is_active())
            .collect();
        // Sort by next_fire ascending — fire earliest first
        due.sort_by(|a, b| {
            a.next_fire.cmp(&b.next_fire)
        });
        due
    }

    /// Jobs due within the lookahead window.
    pub fn upcoming_jobs(
        &self,
        within_seconds: i64,
    ) -> Vec<&ScheduledJob> {
        let now    = chrono::Utc::now();
        let window = now + chrono::Duration::seconds(within_seconds);
        self.jobs.values()
            .filter(|j| j.state.is_active())
            .filter(|j| j.next_fire.map(|f| f <= window).unwrap_or(false))
            .collect()
    }

    pub fn get(&self, job_id: &str) -> Option<&ScheduledJob> {
        self.jobs.get(job_id)
    }

    pub fn get_mut(&mut self, job_id: &str) -> Option<&mut ScheduledJob> {
        self.jobs.get_mut(job_id)
    }

    pub fn len(&self) -> usize  { self.jobs.len() }
    pub fn is_empty(&self) -> bool { self.jobs.is_empty() }

    /// Active jobs count (not cancelled or exhausted).
    pub fn active_count(&self) -> usize {
        self.jobs.values().filter(|j| j.state.is_active()).count()
    }

    /// All job values for iteration.
    pub fn queue_values(&self) -> Vec<&ScheduledJob> {
        self.jobs.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::ScheduledJob;
    use crate::trigger::TriggerType;
    use hydra_soul::TemporalHorizon;

    fn make_job_due() -> ScheduledJob {
        let mut job = ScheduledJob::new(
            "due-job", "action.id", "intent",
            TriggerType::Recurring {
                interval_seconds: 3600,
                first_fire: None,
                label: "test".into(),
            },
            TemporalHorizon::Immediate,
        );
        // Force it to be due
        job.next_fire = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
        job.state     = crate::job::JobState::Pending;
        job
    }

    #[test]
    fn enqueue_and_retrieve() {
        let mut q   = JobQueue::new();
        let job_id  = "job-1".to_string();
        let mut job = make_job_due();
        job.id      = job_id.clone();
        q.enqueue(job).expect("enqueue should succeed");
        assert_eq!(q.len(), 1);
        assert!(q.get(&job_id).is_some());
    }

    #[test]
    fn due_jobs_returned() {
        let mut q = JobQueue::new();
        q.enqueue(make_job_due()).expect("enqueue should succeed");
        let due = q.due_jobs();
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn cancel_marks_job() {
        let mut q   = JobQueue::new();
        let job_id  = "to-cancel".to_string();
        let mut job = make_job_due();
        job.id      = job_id.clone();
        q.enqueue(job).expect("enqueue should succeed");
        q.cancel(&job_id).expect("cancel should succeed");
        assert_eq!(q.get(&job_id).expect("job should exist").state.label(), "cancelled");
        assert_eq!(q.active_count(), 0);
    }

    #[test]
    fn duplicate_job_rejected() {
        let mut q   = JobQueue::new();
        let job_id  = "dup".to_string();
        let mut j1  = make_job_due();
        let mut j2  = make_job_due();
        j1.id = job_id.clone();
        j2.id = job_id.clone();
        q.enqueue(j1).expect("enqueue first should succeed");
        let r = q.enqueue(j2);
        assert!(matches!(r, Err(SchedulerError::DuplicateJob { .. })));
    }
}
