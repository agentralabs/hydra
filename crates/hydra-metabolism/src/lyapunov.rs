//! Lyapunov stability tracking and classification.

use crate::constants::{
    LYAPUNOV_CRITICAL, LYAPUNOV_EMERGENCY, LYAPUNOV_HISTORY_WINDOW, LYAPUNOV_OPTIMAL,
    LYAPUNOV_STABLE,
};
use serde::{Deserialize, Serialize};

/// Classification of system stability based on Lyapunov value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StabilityClass {
    /// System is performing optimally.
    Optimal,
    /// System is stable but not optimal.
    Stable,
    /// System is in alert state — needs attention.
    Alert,
    /// System is critical — immediate intervention needed.
    Critical,
    /// System is in emergency — catastrophic failure imminent.
    Emergency,
}

impl std::fmt::Display for StabilityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Optimal => write!(f, "OPTIMAL"),
            Self::Stable => write!(f, "STABLE"),
            Self::Alert => write!(f, "ALERT"),
            Self::Critical => write!(f, "CRITICAL"),
            Self::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

/// Classify a Lyapunov value into a stability class.
pub fn classify(value: f64) -> StabilityClass {
    if value >= LYAPUNOV_OPTIMAL {
        StabilityClass::Optimal
    } else if value >= LYAPUNOV_STABLE {
        StabilityClass::Stable
    } else if value >= LYAPUNOV_CRITICAL {
        StabilityClass::Alert
    } else if value >= LYAPUNOV_EMERGENCY {
        StabilityClass::Critical
    } else {
        StabilityClass::Emergency
    }
}

/// Tracks Lyapunov values over time for trend analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyapunovTracker {
    /// History of recorded values (most recent last).
    history: Vec<f64>,
    /// Maximum history length.
    window: usize,
}

impl LyapunovTracker {
    /// Create a new tracker with the default window size.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            window: LYAPUNOV_HISTORY_WINDOW,
        }
    }

    /// Record a new Lyapunov value.
    pub fn record(&mut self, value: f64) {
        self.history.push(value);
        if self.history.len() > self.window {
            self.history.remove(0);
        }
    }

    /// Return the most recent Lyapunov value, or `None` if empty.
    pub fn current(&self) -> Option<f64> {
        self.history.last().copied()
    }

    /// Classify the current stability.
    pub fn stability(&self) -> Option<StabilityClass> {
        self.current().map(classify)
    }

    /// Compute the trend (slope) of recent values.
    ///
    /// Positive = improving, negative = degrading.
    /// Returns `None` if fewer than 2 values recorded.
    pub fn trend(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }
        let n = self.history.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = self.mean_inner();
        let mut num = 0.0;
        let mut den = 0.0;
        for (i, &v) in self.history.iter().enumerate() {
            let xi = i as f64 - x_mean;
            num += xi * (v - y_mean);
            den += xi * xi;
        }
        if den.abs() < f64::EPSILON {
            return Some(0.0);
        }
        Some(num / den)
    }

    /// Compute the mean of all recorded values.
    /// Returns `None` if empty.
    pub fn mean(&self) -> Option<f64> {
        if self.history.is_empty() {
            return None;
        }
        Some(self.mean_inner())
    }

    /// Internal mean computation (assumes non-empty).
    fn mean_inner(&self) -> f64 {
        let sum: f64 = self.history.iter().sum();
        sum / self.history.len() as f64
    }

    /// Returns true if the system has been consistently stable
    /// (all values >= LYAPUNOV_STABLE) for at least `min_ticks` ticks.
    pub fn is_consistently_stable(&self, min_ticks: usize) -> bool {
        if self.history.len() < min_ticks {
            return false;
        }
        let tail = &self.history[self.history.len() - min_ticks..];
        tail.iter().all(|&v| v >= LYAPUNOV_STABLE)
    }

    /// Return the number of recorded values.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Return true if no values have been recorded.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

impl Default for LyapunovTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_ranges() {
        assert_eq!(classify(0.5), StabilityClass::Optimal);
        assert_eq!(classify(0.3), StabilityClass::Optimal);
        assert_eq!(classify(0.1), StabilityClass::Stable);
        assert_eq!(classify(0.0), StabilityClass::Stable);
        assert_eq!(classify(-0.1), StabilityClass::Alert);
        assert_eq!(classify(-0.5), StabilityClass::Alert);
        assert_eq!(classify(-0.51), StabilityClass::Critical);
        assert_eq!(classify(-1.0), StabilityClass::Critical);
        assert_eq!(classify(-1.5), StabilityClass::Emergency);
    }

    #[test]
    fn tracker_basic() {
        let mut t = LyapunovTracker::new();
        assert!(t.current().is_none());
        t.record(0.5);
        assert_eq!(t.current(), Some(0.5));
        assert_eq!(t.stability(), Some(StabilityClass::Optimal));
    }

    #[test]
    fn tracker_trend_positive() {
        let mut t = LyapunovTracker::new();
        for i in 0..10 {
            t.record(i as f64 * 0.1);
        }
        let trend = t.trend().expect("trend");
        assert!(trend > 0.0);
    }

    #[test]
    fn tracker_trend_negative() {
        let mut t = LyapunovTracker::new();
        for i in (0..10).rev() {
            t.record(i as f64 * 0.1);
        }
        let trend = t.trend().expect("trend");
        assert!(trend < 0.0);
    }

    #[test]
    fn consistently_stable() {
        let mut t = LyapunovTracker::new();
        for _ in 0..10 {
            t.record(0.5);
        }
        assert!(t.is_consistently_stable(10));
        assert!(!t.is_consistently_stable(11));
    }

    #[test]
    fn window_eviction() {
        let mut t = LyapunovTracker::new();
        for i in 0..150 {
            t.record(i as f64);
        }
        assert_eq!(t.len(), LYAPUNOV_HISTORY_WINDOW);
    }
}
