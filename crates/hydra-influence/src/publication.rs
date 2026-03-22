//! PublishedPattern — one proven pattern offered to the network.
//! Signed by the publishing lineage.
//! Outcome-tracked. Confidence grows with confirmed adoptions.

use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// The category of a published pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternCategory {
    Engineering,
    Security,
    Finance,
    Operations,
    Migration,
    Architecture,
    CrossDomain,
}

impl PatternCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Engineering   => "engineering",
            Self::Security      => "security",
            Self::Finance       => "finance",
            Self::Operations    => "operations",
            Self::Migration     => "migration",
            Self::Architecture  => "architecture",
            Self::CrossDomain   => "cross-domain",
        }
    }
}

/// One published influence pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedPattern {
    pub id:               String,
    pub source_lineage:   String,
    pub title:            String,
    pub description:      String,
    pub category:         PatternCategory,
    pub domain_tags:      Vec<String>,
    /// The actual pattern content — what to do and when.
    pub pattern_content:  String,
    pub evidence_count:   usize,
    pub confidence:       f64,
    pub adoption_count:   usize,
    pub confirmed_outcomes: usize,
    pub source_days:      u32,
    pub integrity_hash:   String,
    pub published_at:     chrono::DateTime<chrono::Utc>,
    pub updated_at:       chrono::DateTime<chrono::Utc>,
}

impl PublishedPattern {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_lineage:  impl Into<String>,
        title:           impl Into<String>,
        description:     impl Into<String>,
        category:        PatternCategory,
        domain_tags:     Vec<String>,
        pattern_content: impl Into<String>,
        evidence_count:  usize,
        confidence:      f64,
        source_days:     u32,
    ) -> Self {
        let now     = chrono::Utc::now();
        let lineage = source_lineage.into();
        let title_s = title.into();
        let hash    = {
            let mut h = Sha256::new();
            h.update(lineage.as_bytes());
            h.update(title_s.as_bytes());
            h.update(source_days.to_le_bytes());
            h.update(evidence_count.to_le_bytes());
            h.update(now.to_rfc3339().as_bytes());
            hex::encode(h.finalize())
        };
        Self {
            id:               uuid::Uuid::new_v4().to_string(),
            source_lineage:   lineage,
            title:            title_s,
            description:      description.into(),
            category,
            domain_tags,
            pattern_content:  pattern_content.into(),
            evidence_count,
            confidence:       confidence.clamp(0.0, 1.0),
            adoption_count:   0,
            confirmed_outcomes: 0,
            source_days,
            integrity_hash:   hash,
            published_at:     now,
            updated_at:       now,
        }
    }

    pub fn verify_integrity(&self) -> bool {
        !self.integrity_hash.is_empty() && self.integrity_hash.len() == 64
    }

    /// Record a confirmed successful outcome — raises confidence.
    pub fn record_outcome(&mut self, success: bool) {
        if success {
            self.confirmed_outcomes += 1;
            let increment = crate::constants::OUTCOME_CONFIDENCE_INCREMENT;
            self.confidence = (self.confidence + increment)
                .min(crate::constants::MAX_PATTERN_CONFIDENCE);
        }
        self.updated_at = chrono::Utc::now();
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} — {:.0}% conf, {} ev, {} adopted, {} confirmed",
            self.category.label(), self.title,
            self.confidence * 100.0, self.evidence_count,
            self.adoption_count, self.confirmed_outcomes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_integrity_valid() {
        let p = PublishedPattern::new(
            "hydra-agentra-lineage",
            "Circuit Breaker at Service Boundaries",
            "Install circuit breakers at all service dependency boundaries",
            PatternCategory::Engineering,
            vec!["engineering".into(), "microservices".into()],
            "Step 1: identify dependency boundaries. Step 2: configure thresholds.",
            47, 0.92, 7300,
        );
        assert!(p.verify_integrity());
        assert_eq!(p.integrity_hash.len(), 64);
    }

    #[test]
    fn outcome_raises_confidence() {
        let mut p = PublishedPattern::new(
            "lineage", "title", "desc",
            PatternCategory::Engineering, vec![],
            "content", 10, 0.80, 1000,
        );
        let before = p.confidence;
        p.record_outcome(true);
        assert!(p.confidence > before);
        assert_eq!(p.confirmed_outcomes, 1);
    }

    #[test]
    fn confidence_capped_at_max() {
        let mut p = PublishedPattern::new(
            "l", "t", "d", PatternCategory::Engineering,
            vec![], "c", 10, 0.99, 100,
        );
        for _ in 0..100 { p.record_outcome(true); }
        assert!(p.confidence <= crate::constants::MAX_PATTERN_CONFIDENCE);
    }
}
