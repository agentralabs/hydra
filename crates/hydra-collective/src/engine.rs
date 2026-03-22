//! CollectiveEngine — the distributed pattern coordinator.

use crate::{
    aggregator::aggregate,
    constants::{MAX_OBSERVATIONS_PER_TOPIC, MAX_STORED_INSIGHTS},
    errors::CollectiveError,
    insight::CollectiveInsight,
    observation::PatternObservation,
};
use std::collections::HashMap;

/// The collective engine.
pub struct CollectiveEngine {
    observations: HashMap<String, Vec<PatternObservation>>,
    insights: Vec<CollectiveInsight>,
}

impl CollectiveEngine {
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            insights: Vec::new(),
        }
    }

    /// Contribute a pattern observation (from any peer, consent checked at caller).
    pub fn contribute(&mut self, observation: PatternObservation) -> Result<(), CollectiveError> {
        let bucket = self
            .observations
            .entry(observation.topic.clone())
            .or_default();

        if bucket.len() >= MAX_OBSERVATIONS_PER_TOPIC {
            bucket.remove(0);
        }
        bucket.push(observation);
        Ok(())
    }

    /// Produce a collective insight for a topic if sufficient observations exist.
    pub fn produce_insight(
        &mut self,
        topic: &str,
        description: &str,
        recommendation: &str,
    ) -> Result<&CollectiveInsight, CollectiveError> {
        let observations: Vec<PatternObservation> = self
            .observations
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();

        let agg = aggregate(topic, &observations)?;

        if self.insights.len() >= MAX_STORED_INSIGHTS {
            self.insights.remove(0);
        }

        let insight = CollectiveInsight::from_aggregated(&agg, description, recommendation);
        self.insights.push(insight);
        Ok(self.insights.last().expect("just pushed"))
    }

    pub fn observation_count_for(&self, topic: &str) -> usize {
        self.observations
            .get(topic)
            .map(|v| v.iter().map(|o| o.count).sum())
            .unwrap_or(0)
    }

    pub fn insight_count(&self) -> usize {
        self.insights.len()
    }

    pub fn topic_count(&self) -> usize {
        self.observations.len()
    }

    /// Summary for TUI.
    pub fn summary(&self) -> String {
        format!(
            "collective: topics={} insights={}",
            self.topic_count(),
            self.insight_count(),
        )
    }
}

impl Default for CollectiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(peer: &str, trust: f64, count: usize) -> PatternObservation {
        PatternObservation::new(
            "cascade-failure",
            peer,
            trust,
            0.85,
            count,
            "desc",
            "engineering",
        )
    }

    #[test]
    fn collective_insight_from_multiple_peers() {
        let mut engine = CollectiveEngine::new();
        engine
            .contribute(obs("peer-a", 0.85, 5))
            .expect("contribute");
        engine
            .contribute(obs("peer-b", 0.78, 8))
            .expect("contribute");
        engine
            .contribute(obs("peer-c", 0.90, 6))
            .expect("contribute");

        let insight = engine
            .produce_insight(
                "cascade-failure",
                "Cascade failures detected across 3 federated instances",
                "Install circuit breakers at all service dependency boundaries",
            )
            .expect("should produce insight");

        assert!(insight.aggregated_confidence >= 0.65);
        assert_eq!(insight.peer_count, 3);
        assert_eq!(engine.insight_count(), 1);
    }

    #[test]
    fn summary_format() {
        let engine = CollectiveEngine::new();
        let s = engine.summary();
        assert!(s.contains("collective:"));
        assert!(s.contains("topics="));
    }
}
