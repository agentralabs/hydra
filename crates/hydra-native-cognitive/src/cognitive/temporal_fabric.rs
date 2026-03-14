//! Temporal Fabric — tracks belief evolution over time.
//! Every belief has a time dimension: when it was created, when it changed,
//! what the pattern of change looks like.
//!
//! Why isn't a sister doing this? Uses Time sister for duration tracking
//! and Memory sister for storage. This module owns the temporal model.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global temporal store — persists across cognitive loop invocations.
pub static GLOBAL_TEMPORAL_STORE: OnceLock<Mutex<TemporalStore>> = OnceLock::new();

/// Get or initialize the global temporal store.
pub fn temporal_store() -> &'static Mutex<TemporalStore> {
    GLOBAL_TEMPORAL_STORE.get_or_init(|| Mutex::new(TemporalStore::new()))
}

/// A snapshot of a belief at a point in time.
#[derive(Debug, Clone)]
pub struct BeliefSnapshot {
    pub content: String,
    pub confidence: f64,
    pub timestamp: String,
    pub source: String,
}

/// Timeline of a belief's evolution.
#[derive(Debug, Clone)]
pub struct BeliefTimeline {
    pub topic: String,
    pub snapshots: Vec<BeliefSnapshot>,
    pub trend: ConfidenceTrend,
    pub volatility: f64,
}

/// Direction of confidence change over time.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceTrend {
    Rising,
    Stable,
    Declining,
    Volatile,
}

/// Store of all belief timelines.
#[derive(Debug, Default)]
pub struct TemporalStore {
    timelines: HashMap<String, BeliefTimeline>,
}

impl TemporalStore {
    pub fn new() -> Self {
        Self { timelines: HashMap::new() }
    }

    /// Record a belief snapshot at the current time.
    pub fn record(&mut self, topic: &str, content: &str, confidence: f64, source: &str) {
        let snapshot = BeliefSnapshot {
            content: content.to_string(),
            confidence,
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: source.to_string(),
        };

        let timeline = self.timelines.entry(topic.to_string()).or_insert_with(|| {
            BeliefTimeline {
                topic: topic.to_string(),
                snapshots: Vec::new(),
                trend: ConfidenceTrend::Stable,
                volatility: 0.0,
            }
        });

        timeline.snapshots.push(snapshot);

        // Recompute trend and volatility
        if timeline.snapshots.len() >= 2 {
            timeline.trend = compute_trend(&timeline.snapshots);
            timeline.volatility = compute_volatility(&timeline.snapshots);
        }
    }

    /// Get the timeline for a specific topic.
    pub fn get_timeline(&self, topic: &str) -> Option<&BeliefTimeline> {
        self.timelines.get(topic)
    }

    /// Find beliefs whose confidence is declining.
    pub fn declining_beliefs(&self) -> Vec<&BeliefTimeline> {
        self.timelines.values()
            .filter(|t| t.trend == ConfidenceTrend::Declining)
            .collect()
    }

    /// Find the most volatile beliefs (likely to change soon).
    pub fn volatile_beliefs(&self, threshold: f64) -> Vec<&BeliefTimeline> {
        self.timelines.values()
            .filter(|t| t.volatility > threshold)
            .collect()
    }

    /// Generate a temporal summary for prompt injection.
    pub fn temporal_summary(&self) -> Option<String> {
        let declining = self.declining_beliefs();
        let volatile = self.volatile_beliefs(0.15);

        if declining.is_empty() && volatile.is_empty() {
            return None;
        }

        let mut summary = String::new();
        if !declining.is_empty() {
            summary.push_str("Beliefs with declining confidence:\n");
            for t in declining.iter().take(3) {
                let latest = t.snapshots.last().map(|s| s.confidence).unwrap_or(0.0);
                summary.push_str(&format!("  {} — now {:.0}%\n", t.topic, latest * 100.0));
            }
        }
        if !volatile.is_empty() {
            summary.push_str("Volatile beliefs (likely to change):\n");
            for t in volatile.iter().take(3) {
                summary.push_str(&format!("  {} — volatility {:.2}\n", t.topic, t.volatility));
            }
        }
        Some(summary)
    }

    /// How many timelines are tracked.
    pub fn timeline_count(&self) -> usize {
        self.timelines.len()
    }
}

/// Compute confidence trend from snapshots.
fn compute_trend(snapshots: &[BeliefSnapshot]) -> ConfidenceTrend {
    if snapshots.len() < 2 {
        return ConfidenceTrend::Stable;
    }

    let recent = &snapshots[snapshots.len().saturating_sub(3)..];
    let diffs: Vec<f64> = recent.windows(2)
        .map(|w| w[1].confidence - w[0].confidence)
        .collect();

    if diffs.is_empty() {
        return ConfidenceTrend::Stable;
    }

    let avg_diff: f64 = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let has_reversals = diffs.windows(2).any(|w| w[0].signum() != w[1].signum());

    if has_reversals && diffs.len() >= 2 {
        ConfidenceTrend::Volatile
    } else if avg_diff > 0.05 {
        ConfidenceTrend::Rising
    } else if avg_diff < -0.05 {
        ConfidenceTrend::Declining
    } else {
        ConfidenceTrend::Stable
    }
}

/// Compute confidence volatility (standard deviation of changes).
fn compute_volatility(snapshots: &[BeliefSnapshot]) -> f64 {
    if snapshots.len() < 2 {
        return 0.0;
    }

    let diffs: Vec<f64> = snapshots.windows(2)
        .map(|w| (w[1].confidence - w[0].confidence).abs())
        .collect();

    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / diffs.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve() {
        let mut store = TemporalStore::new();
        store.record("rust/ownership", "Ownership is good", 0.90, "factory");
        store.record("rust/ownership", "Ownership prevents races", 0.95, "refined");
        let tl = store.get_timeline("rust/ownership").unwrap();
        assert_eq!(tl.snapshots.len(), 2);
    }

    #[test]
    fn test_declining_trend() {
        let snapshots = vec![
            BeliefSnapshot { content: "a".into(), confidence: 0.90, timestamp: "t1".into(), source: "f".into() },
            BeliefSnapshot { content: "b".into(), confidence: 0.80, timestamp: "t2".into(), source: "f".into() },
            BeliefSnapshot { content: "c".into(), confidence: 0.70, timestamp: "t3".into(), source: "f".into() },
        ];
        assert_eq!(compute_trend(&snapshots), ConfidenceTrend::Declining);
    }

    #[test]
    fn test_volatility() {
        let snapshots = vec![
            BeliefSnapshot { content: "a".into(), confidence: 0.90, timestamp: "t1".into(), source: "f".into() },
            BeliefSnapshot { content: "b".into(), confidence: 0.60, timestamp: "t2".into(), source: "f".into() },
            BeliefSnapshot { content: "c".into(), confidence: 0.85, timestamp: "t3".into(), source: "f".into() },
        ];
        let v = compute_volatility(&snapshots);
        assert!(v > 0.0);
    }
}
