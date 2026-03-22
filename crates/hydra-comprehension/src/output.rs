//! Comprehension output types.
//!
//! `ComprehendedInput` is the unified result of the comprehension pipeline.

use crate::domain::Domain;
use crate::resonance::ResonanceResult;
use crate::temporal::TemporalContext;
use hydra_axiom::AxiomPrimitive;
use serde::{Deserialize, Serialize};

/// The source of an input being comprehended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSource {
    /// Direct text from the principal (user).
    PrincipalText,
    /// Transcribed voice input.
    VoiceTranscript,
    /// Output from a sister MCP server.
    SisterOutput {
        /// Name of the sister that produced this output.
        sister_name: String,
    },
    /// Signal from a companion agent.
    CompanionSignal,
    /// Continuous data stream.
    DataStream {
        /// Name of the data source.
        source_name: String,
    },
    /// Error message from a Hydra crate.
    ErrorMessage {
        /// Crate that produced the error.
        crate_name: String,
    },
}

impl InputSource {
    /// Return a human-readable label for this source.
    pub fn label(&self) -> String {
        match self {
            Self::PrincipalText => "principal-text".to_string(),
            Self::VoiceTranscript => "voice-transcript".to_string(),
            Self::SisterOutput { sister_name } => format!("sister:{sister_name}"),
            Self::CompanionSignal => "companion-signal".to_string(),
            Self::DataStream { source_name } => format!("stream:{source_name}"),
            Self::ErrorMessage { crate_name } => format!("error:{crate_name}"),
        }
    }
}

/// The unified output of the comprehension pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehendedInput {
    /// The original raw input text.
    pub raw: String,
    /// The best-matching domain.
    pub primary_domain: Domain,
    /// All detected domains with confidence scores.
    pub all_domains: Vec<(Domain, f64)>,
    /// Extracted axiom primitives.
    pub primitives: Vec<AxiomPrimitive>,
    /// Temporal context (urgency, horizon, constraint status).
    pub temporal: TemporalContext,
    /// Resonance with prior genome entries.
    pub resonance: ResonanceResult,
    /// Where the input came from.
    pub source: InputSource,
    /// Overall confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Whether an LLM was used during comprehension (always false in this phase).
    pub used_llm: bool,
}

impl ComprehendedInput {
    /// Check whether this input is temporally urgent (urgency >= 0.7).
    pub fn is_urgent(&self) -> bool {
        self.temporal.urgency >= 0.7
    }

    /// Return a one-line summary suitable for TUI display.
    pub fn summary(&self) -> String {
        let domain_label = self.primary_domain.label();
        let prim_count = self.primitives.len();
        let urgency = self.temporal.urgency;
        let resonance_count = self.resonance.matches.len();
        let llm_tag = if self.used_llm { "LLM" } else { "pure" };

        format!(
            "[{llm_tag}] domain={domain_label} primitives={prim_count} \
             urgency={urgency:.2} resonance={resonance_count} \
             confidence={conf:.2} source={src}",
            conf = self.confidence,
            src = self.source.label(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resonance::ResonanceResult;
    use crate::temporal::{ConstraintStatus, Horizon, TemporalContext};

    #[test]
    fn summary_format() {
        let ci = ComprehendedInput {
            raw: "test input".to_string(),
            primary_domain: Domain::Engineering,
            all_domains: vec![(Domain::Engineering, 0.5)],
            primitives: vec![AxiomPrimitive::Risk],
            temporal: TemporalContext {
                urgency: 0.8,
                horizon: Horizon::Immediate,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.75,
            used_llm: false,
        };
        let s = ci.summary();
        assert!(s.contains("pure"));
        assert!(s.contains("engineering"));
        assert!(s.contains("confidence=0.75"));
    }

    #[test]
    fn input_source_labels() {
        assert_eq!(InputSource::PrincipalText.label(), "principal-text");
        assert_eq!(
            InputSource::SisterOutput {
                sister_name: "memory".into()
            }
            .label(),
            "sister:memory"
        );
        assert_eq!(
            InputSource::ErrorMessage {
                crate_name: "hydra-kernel".into()
            }
            .label(),
            "error:hydra-kernel"
        );
    }
}
