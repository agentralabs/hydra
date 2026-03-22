//! InfluenceDiscovery — finding relevant patterns.

use crate::{
    constants::MIN_CONFIDENCE_FOR_ADOPTION,
    publication::PublishedPattern,
};
use serde::{Deserialize, Serialize};

/// Query for discovering relevant patterns.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryQuery {
    pub domain:           Option<String>,
    pub category:         Option<String>,
    pub min_confidence:   Option<f64>,
    pub min_evidence:     Option<usize>,
    pub min_source_days:  Option<u32>,
    pub exclude_lineage:  Option<String>,
    pub limit:            Option<usize>,
}

/// One discovery result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub pattern_id:     String,
    pub title:          String,
    pub category:       String,
    pub confidence:     f64,
    pub evidence_count: usize,
    pub source_lineage: String,
    pub source_days:    u32,
    pub relevance:      f64,
}

/// Discover relevant patterns from the registry.
pub fn discover(
    registry:  &[PublishedPattern],
    query:     &DiscoveryQuery,
) -> Vec<DiscoveryResult> {
    let min_conf = query.min_confidence
        .unwrap_or(MIN_CONFIDENCE_FOR_ADOPTION);
    let limit    = query.limit.unwrap_or(20);

    let mut results: Vec<DiscoveryResult> = registry.iter()
        .filter(|p| {
            // Confidence filter
            if p.confidence < min_conf { return false; }
            // Evidence filter
            if let Some(min_ev) = query.min_evidence {
                if p.evidence_count < min_ev { return false; }
            }
            // Source days filter
            if let Some(min_days) = query.min_source_days {
                if p.source_days < min_days { return false; }
            }
            // Exclude own lineage
            if let Some(excl) = &query.exclude_lineage {
                if &p.source_lineage == excl { return false; }
            }
            // Domain filter
            if let Some(domain) = &query.domain {
                if !p.domain_tags.iter().any(|t| t.contains(domain.as_str())) {
                    return false;
                }
            }
            // Category filter
            if let Some(cat) = &query.category {
                if p.category.label() != cat.as_str() { return false; }
            }
            true
        })
        .map(|p| {
            // Relevance: confidence * ln(evidence+1)/10 * ln(source_days+1)/20
            let relevance = p.confidence
                * ((p.evidence_count as f64 + 1.0).ln() / 10.0)
                * ((p.source_days as f64 + 1.0).ln() / 20.0);
            DiscoveryResult {
                pattern_id:     p.id.clone(),
                title:          p.title.clone(),
                category:       p.category.label().to_string(),
                confidence:     p.confidence,
                evidence_count: p.evidence_count,
                source_lineage: p.source_lineage.clone(),
                source_days:    p.source_days,
                relevance:      relevance.min(1.0),
            }
        })
        .collect();

    // Sort by relevance descending
    results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance)
        .unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publication::PatternCategory;

    fn make_pattern(
        lineage: &str, conf: f64, ev: usize, days: u32, domain: &str,
    ) -> PublishedPattern {
        PublishedPattern::new(
            lineage, "Circuit Breaker", "desc",
            PatternCategory::Engineering,
            vec![domain.to_string()],
            "content", ev, conf, days,
        )
    }

    #[test]
    fn discovery_filters_by_confidence() {
        let registry = vec![
            make_pattern("a", 0.90, 20, 3000, "engineering"),
            make_pattern("b", 0.50, 10, 1000, "engineering"),
        ];
        let results = discover(&registry, &DiscoveryQuery::default());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].confidence, 0.90);
    }

    #[test]
    fn discovery_excludes_own_lineage() {
        let registry = vec![
            make_pattern("my-lineage", 0.90, 20, 3000, "engineering"),
            make_pattern("other-lineage", 0.85, 15, 2000, "engineering"),
        ];
        let q = DiscoveryQuery {
            exclude_lineage: Some("my-lineage".into()),
            ..Default::default()
        };
        let results = discover(&registry, &q);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_lineage, "other-lineage");
    }

    #[test]
    fn discovery_sorted_by_relevance() {
        let registry = vec![
            make_pattern("a", 0.75, 5,  500,  "eng"),
            make_pattern("b", 0.92, 50, 7300, "eng"),
        ];
        let results = discover(&registry, &DiscoveryQuery::default());
        assert!(results[0].relevance >= results[1].relevance);
    }
}
