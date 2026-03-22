//! AcquisitionSource — where Hydra goes to fill a knowledge gap.
//! Sources ranked by reliability. Tried in order.

use crate::constants::*;
use serde::{Deserialize, Serialize};

/// A knowledge acquisition source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AcquisitionSource {
    /// AgenticCodebase — 200M+ repos. Primary for code knowledge.
    AgenticCodebase { query: String },
    /// Structured documentation via hydra-reach-extended.
    Documentation { url: String, query: String },
    /// Expert system query via hydra-protocol.
    ExpertSystem { endpoint: String, query: String },
    /// Synthesize from existing belief graph.
    BeliefSynthesis { related_topics: Vec<String> },
    /// Web search via hydra-reach-extended.
    WebSearch { query: String },
    /// Human resolution — last resort.
    HumanResolution { specific_question: String },
}

impl AcquisitionSource {
    pub fn reliability(&self) -> f64 {
        match self {
            Self::AgenticCodebase { .. } => SOURCE_RELIABILITY_AGENTIC_CODEBASE,
            Self::Documentation { .. }   => SOURCE_RELIABILITY_DOCUMENTATION,
            Self::ExpertSystem { .. }    => SOURCE_RELIABILITY_EXPERT_SYSTEM,
            Self::BeliefSynthesis { .. } => SOURCE_RELIABILITY_BELIEF_SYNTHESIS,
            Self::WebSearch { .. }       => SOURCE_RELIABILITY_WEB,
            Self::HumanResolution { .. } => 1.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::AgenticCodebase { .. } => "agentic-codebase",
            Self::Documentation { .. }   => "documentation",
            Self::ExpertSystem { .. }    => "expert-system",
            Self::BeliefSynthesis { .. } => "belief-synthesis",
            Self::WebSearch { .. }       => "web-search",
            Self::HumanResolution { .. } => "human",
        }
    }

    pub fn requires_network(&self) -> bool {
        !matches!(self, Self::BeliefSynthesis { .. } | Self::HumanResolution { .. })
    }

    /// Build the best source for a gap type and topic.
    pub fn for_gap(topic: &str, domain: &str) -> Vec<AcquisitionSource> {
        let lower = topic.to_lowercase();
        let mut sources = Vec::new();

        // Code-related topics -> codebase first
        if lower.contains("implement") || lower.contains("how to")
            || lower.contains("api") || lower.contains("code")
            || domain == "engineering"
        {
            sources.push(Self::AgenticCodebase {
                query: format!("{} {}", topic, domain),
            });
        }

        // Always try documentation
        sources.push(Self::Documentation {
            url:   format!("https://docs.{}.io", domain.replace(' ', "-")),
            query: topic.to_string(),
        });

        // Belief synthesis for structural/relational topics
        sources.push(Self::BeliefSynthesis {
            related_topics: vec![topic.to_string(), domain.to_string()],
        });

        // Web as fallback
        sources.push(Self::WebSearch {
            query: format!("{} {} documentation", topic, domain),
        });

        sources.truncate(MAX_SOURCES_PER_PLAN);
        sources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codebase_most_reliable_for_code() {
        let cb  = AcquisitionSource::AgenticCodebase { query: "test".into() };
        let web = AcquisitionSource::WebSearch { query: "test".into() };
        assert!(cb.reliability() > web.reliability());
    }

    #[test]
    fn human_most_reliable() {
        let h  = AcquisitionSource::HumanResolution {
            specific_question: "?".into(),
        };
        let cb = AcquisitionSource::AgenticCodebase { query: "test".into() };
        assert!(h.reliability() >= cb.reliability());
    }

    #[test]
    fn engineering_gap_gets_codebase_first() {
        let sources = AcquisitionSource::for_gap(
            "how to implement rolling update", "engineering",
        );
        assert_eq!(sources[0].label(), "agentic-codebase");
    }

    #[test]
    fn source_count_bounded() {
        let sources = AcquisitionSource::for_gap("any topic", "any domain");
        assert!(sources.len() <= MAX_SOURCES_PER_PLAN);
    }
}
