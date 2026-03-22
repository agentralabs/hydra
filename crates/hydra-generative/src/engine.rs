//! GenerativeEngine — the full synthesis pipeline.

use crate::compose::{compose, CompositionResult};
use crate::constants::SYNTHESIS_MIN_CONFIDENCE;
use crate::decompose::{decompose, TaskDecomposition};
use crate::errors::GenerativeError;
use crate::gap::detect_gap;
use hydra_axiom::AxiomPrimitive;
use hydra_genome::{ApproachSignature, GenomeStore};
use serde::{Deserialize, Serialize};

/// The outcome of a synthesis attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SynthesisOutcome {
    /// A new capability was successfully synthesized.
    Success {
        /// The name of the synthesized capability.
        capability_name: String,
        /// Confidence in the synthesis.
        confidence: f64,
    },
    /// A gap was detected — specific primitives are needed.
    GapDetected {
        /// What is needed (never says "cannot").
        what_is_needed: String,
        /// Which primitives were covered.
        primitives_covered: Vec<AxiomPrimitive>,
    },
    /// An existing approach was found in the genome.
    ExistingApproach {
        /// The ID of the existing genome entry.
        genome_entry_id: String,
    },
}

/// The generative engine for synthesizing capabilities.
pub struct GenerativeEngine;

impl GenerativeEngine {
    /// Create a new generative engine.
    pub fn new() -> Self {
        Self
    }

    /// Synthesize a capability for a given task description.
    ///
    /// 1. Checks genome for existing matching entries.
    /// 2. If found, returns `ExistingApproach`.
    /// 3. If not, decomposes task into axiom primitives.
    /// 4. If all primitives covered, synthesizes and adds to genome.
    /// 5. If gap detected, returns `GapDetected` (never says "cannot").
    pub fn synthesize_for(
        &self,
        task_description: &str,
        genome_store: &mut GenomeStore,
    ) -> Result<SynthesisOutcome, GenerativeError> {
        if task_description.trim().is_empty() {
            return Err(GenerativeError::EmptyDescription);
        }

        // Step 1: Check genome for existing matches.
        let existing = genome_store.query(task_description);
        if let Some(entry) = existing.first() {
            return Ok(SynthesisOutcome::ExistingApproach {
                genome_entry_id: entry.id.clone(),
            });
        }

        // Step 2: Decompose into primitives.
        let decomposition = decompose(task_description);

        // Step 3: Check if primitives were extracted.
        if decomposition.primitives.is_empty() {
            return self.outcome_from_gap(
                &decomposition,
                "domain-specific primitive extraction needed",
            );
        }

        // Step 4: Compose primitives into a capability.
        let composition = compose(&decomposition);

        if composition.confidence >= SYNTHESIS_MIN_CONFIDENCE {
            self.add_to_genome(
                task_description,
                &decomposition,
                &composition,
                genome_store,
            )
        } else {
            self.outcome_from_gap(
                &decomposition,
                "higher confidence primitives needed",
            )
        }
    }

    /// Convert a gap detection into a `SynthesisOutcome`.
    fn outcome_from_gap(
        &self,
        decomposition: &TaskDecomposition,
        what_is_needed: &str,
    ) -> Result<SynthesisOutcome, GenerativeError> {
        let gap = detect_gap(decomposition, what_is_needed);
        Ok(SynthesisOutcome::GapDetected {
            what_is_needed: gap.what_is_needed,
            primitives_covered: gap.primitives_covered,
        })
    }

    /// Add a synthesized capability to the genome permanently.
    fn add_to_genome(
        &self,
        task_description: &str,
        decomposition: &TaskDecomposition,
        composition: &CompositionResult,
        genome_store: &mut GenomeStore,
    ) -> Result<SynthesisOutcome, GenerativeError> {
        let approach = ApproachSignature::new(
            "synthesized",
            decomposition
                .primitives
                .iter()
                .map(|p| p.label().to_string())
                .collect(),
            vec!["generative-engine".to_string()],
        );

        genome_store
            .add_from_operation(task_description, approach, composition.confidence)
            .map_err(|e| GenerativeError::GenomeAddFailed {
                reason: e.to_string(),
            })?;

        Ok(SynthesisOutcome::Success {
            capability_name: composition.capability_name.clone(),
            confidence: composition.confidence,
        })
    }
}

impl Default for GenerativeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_new_capability() {
        let engine = GenerativeEngine::new();
        let mut store = GenomeStore::new();
        let result = engine
            .synthesize_for("optimize resource allocation under constraints", &mut store)
            .unwrap();
        match result {
            SynthesisOutcome::Success { confidence, .. } => {
                assert!(confidence >= SYNTHESIS_MIN_CONFIDENCE);
            }
            other => panic!("Expected Success, got {:?}", other),
        }
        assert_eq!(store.total_ever(), 1);
    }

    #[test]
    fn existing_approach_found() {
        let engine = GenerativeEngine::new();
        let mut store = GenomeStore::new();

        // Add an entry first.
        store
            .add_from_operation(
                "optimize resource allocation",
                ApproachSignature::new("manual", vec![], vec![]),
                0.8,
            )
            .unwrap();

        let result = engine
            .synthesize_for("optimize resource allocation", &mut store)
            .unwrap();
        match result {
            SynthesisOutcome::ExistingApproach { .. } => {}
            other => panic!("Expected ExistingApproach, got {:?}", other),
        }
    }

    #[test]
    fn empty_description_rejected() {
        let engine = GenerativeEngine::new();
        let mut store = GenomeStore::new();
        let result = engine.synthesize_for("", &mut store);
        assert!(result.is_err());
    }

    #[test]
    fn gap_never_says_cannot() {
        let engine = GenerativeEngine::new();
        let mut store = GenomeStore::new();
        let result = engine.synthesize_for("xyz qqq zzz", &mut store).unwrap();
        match result {
            SynthesisOutcome::GapDetected { what_is_needed, .. } => {
                assert!(!what_is_needed.contains("cannot"));
                assert!(!what_is_needed.contains("unable"));
            }
            other => panic!("Expected GapDetected, got {:?}", other),
        }
    }
}
