//! CollectiveInsight — the emergent finding from aggregated observations.

use crate::aggregator::AggregatedPattern;
use serde::{Deserialize, Serialize};

/// A collective insight produced from federated pattern observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveInsight {
    pub id: String,
    pub topic: String,
    pub description: String,
    pub aggregated_confidence: f64,
    pub total_observations: usize,
    pub peer_count: usize,
    pub contributing_peers: Vec<String>,
    pub domains: Vec<String>,
    pub recommendation: String,
    pub produced_at: chrono::DateTime<chrono::Utc>,
}

impl CollectiveInsight {
    pub fn from_aggregated(
        agg: &AggregatedPattern,
        description: impl Into<String>,
        recommendation: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: agg.topic.clone(),
            description: description.into(),
            aggregated_confidence: agg.aggregated_confidence,
            total_observations: agg.total_observations,
            peer_count: agg.peer_count,
            contributing_peers: agg.contributing_peers.clone(),
            domains: agg.domains.clone(),
            recommendation: recommendation.into(),
            produced_at: chrono::Utc::now(),
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] conf={:.2} peers={} obs={} → {}",
            self.topic,
            self.aggregated_confidence,
            self.peer_count,
            self.total_observations,
            &self.recommendation[..self.recommendation.len().min(60)],
        )
    }
}
