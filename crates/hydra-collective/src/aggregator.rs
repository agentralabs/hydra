//! ObservationAggregator — merges observations from multiple peers.
//! Trust-weighted. Consent-gated (at calling layer).

use crate::{constants::*, errors::CollectiveError, observation::PatternObservation};

/// Aggregated statistics for one pattern topic.
#[derive(Debug, Clone)]
pub struct AggregatedPattern {
    pub topic: String,
    pub total_observations: usize,
    pub peer_count: usize,
    pub aggregated_confidence: f64,
    pub domains: Vec<String>,
    pub contributing_peers: Vec<String>,
}

impl AggregatedPattern {
    pub fn is_sufficient(&self) -> bool {
        self.total_observations >= MIN_OBSERVATIONS_FOR_INSIGHT
            && self.aggregated_confidence >= MIN_INSIGHT_CONFIDENCE
    }
}

/// Aggregates observations across peers.
pub fn aggregate(
    topic: &str,
    observations: &[PatternObservation],
) -> Result<AggregatedPattern, CollectiveError> {
    let relevant: Vec<&PatternObservation> =
        observations.iter().filter(|o| o.topic == topic).collect();

    let total_count: usize = relevant.iter().map(|o| o.count).sum();

    if total_count < MIN_OBSERVATIONS_FOR_INSIGHT {
        return Err(CollectiveError::InsufficientObservations {
            topic: topic.to_string(),
            count: total_count,
            min: MIN_OBSERVATIONS_FOR_INSIGHT,
        });
    }

    // Weighted average confidence
    let total_weight: f64 = relevant
        .iter()
        .map(|o| o.peer_trust.powf(TRUST_WEIGHT_EXPONENT) * o.count as f64)
        .sum();

    let agg_conf = if total_weight > 1e-10 {
        relevant
            .iter()
            .map(|o| o.confidence * o.peer_trust.powf(TRUST_WEIGHT_EXPONENT) * o.count as f64)
            .sum::<f64>()
            / total_weight
    } else {
        0.0
    };

    if agg_conf < MIN_INSIGHT_CONFIDENCE {
        return Err(CollectiveError::LowConfidence {
            confidence: agg_conf,
            min: MIN_INSIGHT_CONFIDENCE,
        });
    }

    let mut domains: Vec<String> = relevant
        .iter()
        .map(|o| o.domain.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    domains.sort();

    let contributing_peers: Vec<String> = relevant
        .iter()
        .map(|o| o.peer_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    Ok(AggregatedPattern {
        topic: topic.to_string(),
        total_observations: total_count,
        peer_count: contributing_peers.len(),
        aggregated_confidence: agg_conf,
        domains,
        contributing_peers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(peer: &str, trust: f64, conf: f64, count: usize) -> PatternObservation {
        PatternObservation::new(
            "cascade-failure",
            peer,
            trust,
            conf,
            count,
            "desc",
            "engineering",
        )
    }

    #[test]
    fn aggregation_succeeds_with_enough_observations() {
        let observations = vec![
            obs("peer-a", 0.85, 0.82, 5),
            obs("peer-b", 0.78, 0.79, 8),
            obs("peer-c", 0.90, 0.91, 6),
        ];
        let result = aggregate("cascade-failure", &observations).expect("should aggregate");
        assert!(result.total_observations >= MIN_OBSERVATIONS_FOR_INSIGHT);
        assert!(result.aggregated_confidence >= MIN_INSIGHT_CONFIDENCE);
        assert_eq!(result.peer_count, 3);
    }

    #[test]
    fn insufficient_observations_errors() {
        let observations = vec![obs("peer-a", 0.8, 0.9, 1)];
        let r = aggregate("cascade-failure", &observations);
        assert!(matches!(
            r,
            Err(CollectiveError::InsufficientObservations { .. })
        ));
    }

    #[test]
    fn high_trust_peers_weight_more() {
        // Mix a high-trust high-conf peer with a low-trust low-conf peer
        // vs a low-trust high-conf peer with a high-trust low-conf peer
        // The first mix should have higher aggregated confidence
        let high_trust_high_conf = vec![obs("peer-a", 0.95, 0.92, 5), obs("peer-b", 0.30, 0.60, 5)];
        let low_trust_high_conf = vec![obs("peer-a", 0.30, 0.92, 5), obs("peer-b", 0.95, 0.60, 5)];

        if let (Ok(h), Ok(l)) = (
            aggregate("cascade-failure", &high_trust_high_conf),
            aggregate("cascade-failure", &low_trust_high_conf),
        ) {
            // When the high-trust peer has high confidence, aggregated should be higher
            assert!(h.aggregated_confidence > l.aggregated_confidence);
        }
    }
}
