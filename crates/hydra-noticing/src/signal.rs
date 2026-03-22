//! NoticingSignal — one observation produced by ambient watching.
//! These are what Hydra generates without being asked.

use serde::{Deserialize, Serialize};

/// What kind of pattern was noticed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoticingKind {
    /// A metric is drifting from its baseline.
    MetricDrift {
        metric:    String,
        direction: DriftDirection,
        magnitude: f64,
        weeks:     u32,
    },
    /// A recurring pattern has broken.
    PatternBreak {
        pattern:       String,
        last_occurred: chrono::DateTime<chrono::Utc>,
        days_absent:   u64,
    },
    /// Multiple small issues are compounding into a larger risk.
    CompoundRisk {
        issue_count:  usize,
        shared_theme: String,
    },
    /// Something expected to happen by now hasn't.
    TemporalGap {
        expected:   String,
        overdue_by: String,
    },
    /// Activity has drifted from what matters most (orientation).
    OrientationDrift {
        current_focus: String,
        orientation:   String,
    },
}

/// Direction of drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriftDirection {
    Increasing,
    Decreasing,
}

impl DriftDirection {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Increasing => "increasing",
            Self::Decreasing => "decreasing",
        }
    }
}

/// One noticing signal — what Hydra noticed without being asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticingSignal {
    pub id:           String,
    pub kind:         NoticingKind,
    pub significance: f64,
    pub narrative:    String,
    pub action_hint:  Option<String>,
    pub noticed_at:   chrono::DateTime<chrono::Utc>,
    pub surfaced:     bool,
}

impl NoticingSignal {
    pub fn new(
        kind:        NoticingKind,
        significance: f64,
        narrative:   impl Into<String>,
        action_hint: Option<String>,
    ) -> Self {
        Self {
            id:           uuid::Uuid::new_v4().to_string(),
            kind,
            significance: significance.clamp(0.0, 1.0),
            narrative:    narrative.into(),
            action_hint,
            noticed_at:   chrono::Utc::now(),
            surfaced:     false,
        }
    }

    pub fn is_significant(&self) -> bool {
        self.significance >= crate::constants::SIGNAL_SIGNIFICANCE_FLOOR
    }

    pub fn mark_surfaced(&mut self) {
        self.surfaced = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn significant_signal_above_floor() {
        let s = NoticingSignal::new(
            NoticingKind::MetricDrift {
                metric: "latency".into(),
                direction: DriftDirection::Increasing,
                magnitude: 0.15,
                weeks: 4,
            },
            0.75,
            "Latency drifting up",
            None,
        );
        assert!(s.is_significant());
    }

    #[test]
    fn below_floor_not_significant() {
        let s = NoticingSignal::new(
            NoticingKind::MetricDrift {
                metric: "noise".into(),
                direction: DriftDirection::Increasing,
                magnitude: 0.01,
                weeks: 1,
            },
            0.2,
            "Minor drift",
            None,
        );
        assert!(!s.is_significant());
    }

    #[test]
    fn surfacing_marks_signal() {
        let mut s = NoticingSignal::new(
            NoticingKind::PatternBreak {
                pattern:       "daily deployment".into(),
                last_occurred: chrono::Utc::now(),
                days_absent:   8,
            },
            0.6,
            "Pattern broken",
            None,
        );
        assert!(!s.surfaced);
        s.mark_surfaced();
        assert!(s.surfaced);
    }
}
