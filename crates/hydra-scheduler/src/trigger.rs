//! TriggerType — when does a job fire?
//! Three types: recurring schedule, constraint activation, one-shot future.

use serde::{Deserialize, Serialize};

/// When a scheduled job fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerType {
    /// Fires on a recurring schedule.
    /// interval_seconds: how often to fire.
    /// first_fire: when to fire first (None = next interval from now).
    Recurring {
        interval_seconds: u64,
        first_fire:       Option<chrono::DateTime<chrono::Utc>>,
        label:            String,
    },

    /// Fires once at a specific future time.
    OneShot {
        fire_at: chrono::DateTime<chrono::Utc>,
    },

    /// Fires when a temporal constraint activates.
    /// constraint_id: the ID in hydra-temporal's constraint graph.
    ConstraintActivation {
        constraint_id:    String,
        constraint_label: String,
    },

    /// Fires when a metric condition is met.
    MetricCondition {
        metric:    String,
        condition: MetricConditionType,
        label:     String,
    },
}

/// The type of metric condition that triggers a job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricConditionType {
    EqualsZero,
    ExceedsThreshold { threshold: f64 },
    DropsBelow { threshold: f64 },
    StaysZeroFor { duration_hours: u64 },
}

impl TriggerType {
    pub fn label(&self) -> String {
        match self {
            Self::Recurring { label, .. }         => format!("recurring:{}", label),
            Self::OneShot { fire_at }             => format!("once:{}", fire_at.format("%Y-%m-%d")),
            Self::ConstraintActivation { constraint_label, .. }
                                                  => format!("constraint:{}", constraint_label),
            Self::MetricCondition { label, .. }   => format!("metric:{}", label),
        }
    }

    /// For recurring triggers: compute the next fire time after `since`.
    pub fn next_fire_after(
        &self,
        since: &chrono::DateTime<chrono::Utc>,
        last_fired: Option<&chrono::DateTime<chrono::Utc>>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        match self {
            Self::Recurring { interval_seconds, first_fire, .. } => {
                if let Some(last) = last_fired {
                    let next = *last + chrono::Duration::seconds(*interval_seconds as i64);
                    Some(next)
                } else if let Some(first) = first_fire {
                    if first > since { Some(*first) } else { Some(*since) }
                } else {
                    Some(*since + chrono::Duration::seconds(*interval_seconds as i64))
                }
            }
            Self::OneShot { fire_at } => {
                if fire_at > since { Some(*fire_at) } else { None }
            }
            // Constraint and metric triggers are event-driven, not time-driven
            Self::ConstraintActivation { .. } | Self::MetricCondition { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurring_next_fire_from_now() {
        let trigger = TriggerType::Recurring {
            interval_seconds: 3600, // hourly
            first_fire: None,
            label: "hourly-job".into(),
        };
        let now  = chrono::Utc::now();
        let next = trigger.next_fire_after(&now, None).expect("should have next fire");
        let diff = (next - now).num_seconds();
        assert!(diff >= 3590 && diff <= 3610);
    }

    #[test]
    fn recurring_next_fire_from_last() {
        let trigger = TriggerType::Recurring {
            interval_seconds: 3600,
            first_fire: None,
            label: "test".into(),
        };
        let last = chrono::Utc::now() - chrono::Duration::seconds(1800);
        let now  = chrono::Utc::now();
        let next = trigger.next_fire_after(&now, Some(&last)).expect("should have next fire");
        let diff = (next - last).num_seconds();
        assert_eq!(diff, 3600);
    }

    #[test]
    fn one_shot_past_returns_none() {
        let trigger = TriggerType::OneShot {
            fire_at: chrono::Utc::now() - chrono::Duration::hours(1),
        };
        let now = chrono::Utc::now();
        assert!(trigger.next_fire_after(&now, None).is_none());
    }

    #[test]
    fn one_shot_future_returns_fire_at() {
        let future = chrono::Utc::now() + chrono::Duration::hours(24);
        let trigger = TriggerType::OneShot { fire_at: future };
        let now = chrono::Utc::now();
        let next = trigger.next_fire_after(&now, None).expect("should have next fire");
        assert_eq!(next, future);
    }

    #[test]
    fn trigger_labels_non_empty() {
        let labels = vec![
            TriggerType::Recurring {
                interval_seconds: 60, first_fire: None,
                label: "test".into(),
            }.label(),
            TriggerType::OneShot {
                fire_at: chrono::Utc::now(),
            }.label(),
            TriggerType::ConstraintActivation {
                constraint_id: "c1".into(),
                constraint_label: "freeze-lift".into(),
            }.label(),
        ];
        for l in labels {
            assert!(!l.is_empty());
        }
    }
}
