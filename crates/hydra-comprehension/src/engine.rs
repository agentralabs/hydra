//! The comprehension engine — the 4-stage pipeline.
//!
//! Lifts raw input to structured meaning via:
//! 1. Domain detection
//! 2. Primitive mapping
//! 3. Temporal placement
//! 4. Memory resonance
//!
//! Zero LLM calls. Pure structural analysis.

use crate::constants::{
    COMPREHENSION_CONFIDENCE_THRESHOLD, MAX_PRIMITIVES_PER_INPUT, MIN_SUBSTANTIAL_TOKEN_COUNT,
};
use crate::domain::DomainVocabulary;
use crate::errors::ComprehensionError;
use crate::output::{ComprehendedInput, InputSource};
use crate::primitive::PrimitiveMapping;
use crate::resonance::MemoryResonance;
use crate::temporal::TemporalPlacement;
use hydra_genome::GenomeStore;

/// The comprehension engine. Stateless — all state lives in the genome store.
pub struct ComprehensionEngine {
    /// Domain vocabulary for stage 1.
    vocab: DomainVocabulary,
}

impl ComprehensionEngine {
    /// Create a new comprehension engine.
    pub fn new() -> Self {
        Self {
            vocab: DomainVocabulary::new(),
        }
    }

    /// Comprehend an input from a given source.
    ///
    /// Runs the 4-stage pipeline and returns a `ComprehendedInput`.
    /// Returns an error if the input is empty or below minimum length.
    pub fn comprehend(
        &self,
        input: &str,
        source: InputSource,
        genome: &GenomeStore,
    ) -> Result<ComprehendedInput, ComprehensionError> {
        self.validate(input)?;

        // Stage 1: Domain detection
        let all_domains = self.vocab.detect(input);
        let primary_domain = all_domains
            .first()
            .map(|(d, _)| d.clone())
            .unwrap_or(crate::domain::Domain::Unknown);
        let domain_confidence = all_domains.first().map(|(_, c)| *c).unwrap_or(0.0);

        // Stage 2: Primitive mapping
        let primitives = PrimitiveMapping::extract(input);
        let primitive_fraction = primitives.len() as f64 / MAX_PRIMITIVES_PER_INPUT as f64;

        // Stage 3: Temporal placement
        let temporal = TemporalPlacement::analyze(input);
        let temporal_factor = temporal.urgency;

        // Stage 4: Memory resonance
        let resonance = MemoryResonance::check_resonance(input, genome);
        let resonance_factor = if resonance.has_prior_context {
            resonance
                .matches
                .first()
                .map(|m| m.similarity)
                .unwrap_or(0.0)
        } else {
            0.0
        };

        // Overall confidence
        let confidence = 0.3 * domain_confidence
            + 0.3 * primitive_fraction
            + 0.2 * temporal_factor
            + 0.2 * resonance_factor;

        Ok(ComprehendedInput {
            raw: input.to_string(),
            primary_domain,
            all_domains,
            primitives,
            temporal,
            resonance,
            source,
            confidence,
            used_llm: false,
        })
    }

    /// Comprehend output from a sister MCP server.
    ///
    /// Convenience wrapper that sets the source to `SisterOutput`.
    pub fn comprehend_sister(
        &self,
        input: &str,
        sister_name: &str,
        genome: &GenomeStore,
    ) -> Result<ComprehendedInput, ComprehensionError> {
        let source = InputSource::SisterOutput {
            sister_name: sister_name.to_string(),
        };
        self.comprehend(input, source, genome)
    }

    /// Validate input before processing.
    fn validate(&self, input: &str) -> Result<(), ComprehensionError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(ComprehensionError::EmptyInput);
        }

        let token_count = trimmed.split_whitespace().count();
        if token_count < MIN_SUBSTANTIAL_TOKEN_COUNT {
            return Err(ComprehensionError::BelowMinimumLength {
                actual: token_count,
                minimum: MIN_SUBSTANTIAL_TOKEN_COUNT,
            });
        }

        Ok(())
    }
}

impl Default for ComprehensionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether a comprehension result meets the confidence threshold.
pub fn meets_threshold(result: &ComprehendedInput) -> bool {
    result.confidence >= COMPREHENSION_CONFIDENCE_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine.comprehend("", InputSource::PrincipalText, &genome);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_short() {
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine.comprehend("hi", InputSource::PrincipalText, &genome);
        assert!(result.is_err());
    }

    #[test]
    fn comprehends_engineering() {
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine
            .comprehend(
                "deploy the api service to docker container",
                InputSource::PrincipalText,
                &genome,
            )
            .expect("should comprehend");
        assert_eq!(result.primary_domain, crate::domain::Domain::Engineering);
        assert!(!result.used_llm);
    }

    #[test]
    fn sister_output_tagged() {
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine
            .comprehend_sister("deploy the api service now", "memory", &genome)
            .expect("should comprehend");
        match &result.source {
            InputSource::SisterOutput { sister_name } => {
                assert_eq!(sister_name, "memory");
            }
            _ => panic!("expected SisterOutput"),
        }
    }

    #[test]
    fn zero_llm_always() {
        let engine = ComprehensionEngine::new();
        let genome = GenomeStore::new();
        let result = engine
            .comprehend(
                "critical risk in the deploy pipeline now",
                InputSource::PrincipalText,
                &genome,
            )
            .expect("should comprehend");
        assert!(!result.used_llm);
    }
}
