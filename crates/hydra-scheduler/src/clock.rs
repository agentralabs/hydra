//! SchedulerClock — time evaluation for the scheduler.
//! Centralizes "is it time yet?" logic.

use crate::constants::LOOKAHEAD_SECONDS;
use chrono::{DateTime, Utc};

/// Evaluates temporal conditions for the scheduler.
pub struct SchedulerClock;

impl SchedulerClock {
    pub fn now() -> DateTime<Utc> { Utc::now() }

    /// True if `fire_at` is in the past or within the lookahead window.
    pub fn is_due(fire_at: &DateTime<Utc>) -> bool {
        let now     = Self::now();
        let window  = now + chrono::Duration::seconds(LOOKAHEAD_SECONDS);
        *fire_at <= window
    }

    /// Seconds until `fire_at`.
    pub fn seconds_until(fire_at: &DateTime<Utc>) -> i64 {
        let now = Self::now();
        (*fire_at - now).num_seconds()
    }

    /// Human-readable "fires in X" string.
    pub fn human_until(fire_at: &DateTime<Utc>) -> String {
        let secs = Self::seconds_until(fire_at);
        if secs <= 0 {
            "overdue".into()
        } else if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else if secs < 86400 {
            format!("{}h", secs / 3600)
        } else {
            format!("{}d", secs / 86400)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn past_time_is_due() {
        let past = Utc::now() - chrono::Duration::hours(1);
        assert!(SchedulerClock::is_due(&past));
    }

    #[test]
    fn future_time_not_due() {
        let future = Utc::now() + chrono::Duration::hours(24);
        assert!(!SchedulerClock::is_due(&future));
    }

    #[test]
    fn human_until_overdue() {
        let past = Utc::now() - chrono::Duration::seconds(10);
        assert_eq!(SchedulerClock::human_until(&past), "overdue");
    }

    #[test]
    fn human_until_hours() {
        let future = Utc::now() + chrono::Duration::hours(3);
        let h = SchedulerClock::human_until(&future);
        assert!(h.ends_with('h'));
    }

    #[test]
    fn human_until_days() {
        let future = Utc::now() + chrono::Duration::days(5);
        let h = SchedulerClock::human_until(&future);
        assert!(h.ends_with('d'));
    }
}
