//! Anomaly context — tracks unexpected patterns Hydra has detected.

use crate::constants::{ANOMALY_CONFIDENCE_THRESHOLD, MAX_ACTIVE_ANOMALIES};
use crate::window::{ContextItem, ContextWindow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A signal indicating an anomaly Hydra has detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalySignal {
    /// Description of the anomaly.
    pub description: String,
    /// Confidence that this is a genuine anomaly (0.0 to 1.0).
    pub confidence: f64,
    /// Domain where the anomaly was detected, if any.
    pub domain: Option<String>,
    /// When this anomaly was detected.
    pub detected_at: DateTime<Utc>,
}

impl AnomalySignal {
    /// Create a new anomaly signal.
    pub fn new(description: impl Into<String>, confidence: f64) -> Self {
        Self {
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            domain: None,
            detected_at: Utc::now(),
        }
    }

    /// Create an anomaly signal with a domain tag.
    pub fn with_domain(
        description: impl Into<String>,
        confidence: f64,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            domain: Some(domain.into()),
            detected_at: Utc::now(),
        }
    }
}

/// Tracks active anomaly signals.
#[derive(Debug, Clone)]
pub struct AnomalyContext {
    /// Active anomaly signals.
    anomalies: Vec<AnomalySignal>,
}

impl AnomalyContext {
    /// Create a new empty anomaly context.
    pub fn new() -> Self {
        Self {
            anomalies: Vec::new(),
        }
    }

    /// Add an anomaly signal if it meets the confidence threshold.
    ///
    /// Anomalies below the threshold are silently ignored.
    /// If at capacity, the least confident anomaly is replaced.
    pub fn add_anomaly(&mut self, anomaly: AnomalySignal) {
        if anomaly.confidence < ANOMALY_CONFIDENCE_THRESHOLD {
            return;
        }

        if self.anomalies.len() < MAX_ACTIVE_ANOMALIES {
            self.anomalies.push(anomaly);
        } else if let Some(min_idx) = self
            .anomalies
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
        {
            if self.anomalies[min_idx].confidence < anomaly.confidence {
                self.anomalies[min_idx] = anomaly;
            }
        }
    }

    /// Return the number of active anomalies.
    pub fn len(&self) -> usize {
        self.anomalies.len()
    }

    /// Check whether there are no active anomalies.
    pub fn is_empty(&self) -> bool {
        self.anomalies.is_empty()
    }

    /// Build a context window from active anomalies.
    pub fn build_window(&self) -> ContextWindow {
        let mut window = ContextWindow::new("anomalies");
        for anomaly in &self.anomalies {
            let item = match &anomaly.domain {
                Some(d) => ContextItem::with_domain(&anomaly.description, anomaly.confidence, d),
                None => ContextItem::new(&anomaly.description, anomaly.confidence),
            };
            window.add(item);
        }
        window
    }
}

impl Default for AnomalyContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::ANOMALY_CONFIDENCE_THRESHOLD;

    #[test]
    fn rejects_below_threshold() {
        let mut ctx = AnomalyContext::new();
        ctx.add_anomaly(AnomalySignal::new(
            "weak anomaly",
            ANOMALY_CONFIDENCE_THRESHOLD - 0.1,
        ));
        assert!(ctx.is_empty());
    }

    #[test]
    fn accepts_above_threshold() {
        let mut ctx = AnomalyContext::new();
        ctx.add_anomaly(AnomalySignal::new(
            "strong anomaly",
            ANOMALY_CONFIDENCE_THRESHOLD + 0.1,
        ));
        assert_eq!(ctx.len(), 1);
    }

    #[test]
    fn respects_max_active() {
        let mut ctx = AnomalyContext::new();
        for i in 0..15 {
            ctx.add_anomaly(AnomalySignal::new(
                format!("anomaly-{i}"),
                ANOMALY_CONFIDENCE_THRESHOLD + (i as f64) * 0.02,
            ));
        }
        assert!(ctx.len() <= MAX_ACTIVE_ANOMALIES);
    }

    #[test]
    fn build_window_creates_items() {
        let mut ctx = AnomalyContext::new();
        ctx.add_anomaly(AnomalySignal::new("unusual pattern", 0.8));
        let window = ctx.build_window();
        assert_eq!(window.len(), 1);
    }
}
