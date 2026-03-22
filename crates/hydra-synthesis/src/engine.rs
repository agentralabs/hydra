//! Synthesis engine — cross-domain pattern discovery.

use crate::constants::{MAX_SYNTHESIS_INSIGHTS, MAX_PATTERN_LIBRARY, SYNTHESIS_CONFIDENCE_FLOOR};
use crate::errors::SynthesisError;
use crate::insight::SynthesisInsight;
use crate::matcher::find_cross_domain_matches;
use crate::pattern::StructuralPattern;
use hydra_comprehension::ComprehendedInput;
use hydra_reasoning::ReasoningResult;
use std::collections::HashSet;

/// The synthesis engine. Discovers cross-domain patterns from axiom primitives.
///
/// This engine never calls the LLM — all insights are derived from structural
/// pattern matching over axiom primitives and genome patterns.
#[derive(Debug)]
pub struct SynthesisEngine {
    /// The pattern library.
    library: Vec<StructuralPattern>,
}

impl SynthesisEngine {
    /// Create a new synthesis engine with an empty library.
    pub fn new() -> Self {
        Self {
            library: Vec::new(),
        }
    }

    /// Ingest a comprehended input and reasoning result, extracting patterns.
    ///
    /// Returns an error if the library is at capacity.
    pub fn ingest(
        &mut self,
        input: &ComprehendedInput,
        result: &ReasoningResult,
    ) -> Result<(), SynthesisError> {
        if self.library.len() >= MAX_PATTERN_LIBRARY {
            return Err(SynthesisError::LibraryAtCapacity {
                max: MAX_PATTERN_LIBRARY,
            });
        }

        if input.primitives.is_empty() {
            return Err(SynthesisError::InsufficientPrimitives { have: 0 });
        }

        let description = build_pattern_description(input, result);
        let pattern = StructuralPattern::from_primitives(
            input.primary_domain.label(),
            &input.primitives,
            description,
        );
        self.library.push(pattern);
        Ok(())
    }

    /// Run synthesis: find cross-domain patterns and generate insights.
    ///
    /// Returns insights for cross-domain matches above the confidence threshold.
    pub fn synthesize(
        &self,
        _input: &ComprehendedInput,
        _result: &ReasoningResult,
    ) -> Result<Vec<SynthesisInsight>, SynthesisError> {
        if self.library.is_empty() {
            return Err(SynthesisError::NoStructuralPatterns);
        }

        let matches = find_cross_domain_matches(&self.library);

        let insights: Vec<SynthesisInsight> = matches
            .iter()
            .filter(|m| m.similarity >= SYNTHESIS_CONFIDENCE_FLOOR)
            .take(MAX_SYNTHESIS_INSIGHTS)
            .map(SynthesisInsight::from_match)
            .collect();

        Ok(insights)
    }

    /// Return the current library size.
    pub fn library_size(&self) -> usize {
        self.library.len()
    }

    /// Return the number of unique domains in the library.
    pub fn unique_domains(&self) -> usize {
        let domains: HashSet<&str> = self.library.iter().map(|p| p.domain.as_str()).collect();
        domains.len()
    }

    /// Return a TUI-friendly summary.
    pub fn summary(&self) -> String {
        format!(
            "synthesis: library={} domains={}",
            self.library.len(),
            self.unique_domains(),
        )
    }
}

impl Default for SynthesisEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a description for a pattern from input and result.
fn build_pattern_description(input: &ComprehendedInput, result: &ReasoningResult) -> String {
    let primary_text = result
        .primary
        .as_ref()
        .map(|c| c.statement.as_str())
        .unwrap_or("no conclusion");
    format!(
        "domain={} input=\"{}\" primary=\"{}\"",
        input.primary_domain.label(),
        truncate(&input.raw, 40),
        truncate(primary_text, 40),
    )
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s
            .char_indices()
            .nth(max_len)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_axiom::AxiomPrimitive;
    use hydra_comprehension::*;
    use hydra_reasoning::conclusion::{ReasoningConclusion, ReasoningMode};

    fn make_input(domain: Domain, primitives: Vec<AxiomPrimitive>) -> ComprehendedInput {
        ComprehendedInput {
            raw: "test synthesis input text".into(),
            primary_domain: domain.clone(),
            all_domains: vec![(domain, 0.5)],
            primitives,
            temporal: TemporalContext {
                urgency: 0.5,
                horizon: Horizon::Immediate,
                constraint_status: ConstraintStatus::None,
            },
            resonance: ResonanceResult::empty(),
            source: InputSource::PrincipalText,
            confidence: 0.7,
            used_llm: false,
        }
    }

    fn make_result() -> ReasoningResult {
        let c = ReasoningConclusion::new(
            ReasoningMode::Deductive,
            "test conclusion",
            0.8,
            vec![],
            false,
        );
        ReasoningResult {
            conclusions: vec![c.clone()],
            synthesis_confidence: 0.8,
            used_llm: false,
            active_modes: 1,
            primary: Some(c),
            mode_summary: vec![("deductive".to_string(), true)],
        }
    }

    #[test]
    fn ingest_adds_to_library() {
        let mut engine = SynthesisEngine::new();
        let input = make_input(
            Domain::Engineering,
            vec![AxiomPrimitive::Risk, AxiomPrimitive::CausalLink],
        );
        let result = make_result();
        assert!(engine.ingest(&input, &result).is_ok());
        assert_eq!(engine.library_size(), 1);
    }

    #[test]
    fn empty_primitives_rejected() {
        let mut engine = SynthesisEngine::new();
        let input = make_input(Domain::Engineering, vec![]);
        let result = make_result();
        assert!(engine.ingest(&input, &result).is_err());
    }

    #[test]
    fn summary_format() {
        let engine = SynthesisEngine::new();
        let s = engine.summary();
        assert!(s.contains("synthesis:"));
        assert!(s.contains("library="));
        assert!(s.contains("domains="));
    }
}
