//! Memory resonance — checks for prior context in the genome store.
//!
//! Uses `SituationSignature` Jaccard similarity to find matching
//! genome entries. No LLM calls.

use crate::constants::{RESONANCE_SIMILARITY_THRESHOLD, RESONANCE_TOP_N};
use hydra_genome::{GenomeStore, SituationSignature};
use serde::{Deserialize, Serialize};

/// A single resonance match from the genome store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceMatch {
    /// The genome entry ID that matched.
    pub entry_id: String,
    /// Jaccard similarity score.
    pub similarity: f64,
}

/// Result of a resonance check against the genome store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceResult {
    /// Matching genome entries above the similarity threshold.
    pub matches: Vec<ResonanceMatch>,
    /// Whether any prior context was found.
    pub has_prior_context: bool,
}

impl ResonanceResult {
    /// Create an empty resonance result (no matches).
    pub fn empty() -> Self {
        Self {
            matches: Vec::new(),
            has_prior_context: false,
        }
    }
}

/// Memory resonance checker.
pub struct MemoryResonance;

impl MemoryResonance {
    /// Check the genome store for prior context matching the input.
    ///
    /// Builds a `SituationSignature` from the input and compares it
    /// against all genome entries. Returns entries whose similarity
    /// exceeds `RESONANCE_SIMILARITY_THRESHOLD`, capped at `RESONANCE_TOP_N`.
    pub fn check_resonance(input: &str, genome: &GenomeStore) -> ResonanceResult {
        if genome.is_empty() {
            return ResonanceResult::empty();
        }

        let query_sig = SituationSignature::from_description(input);
        let candidates = genome.query(input);

        let mut matches: Vec<ResonanceMatch> = candidates
            .into_iter()
            .filter_map(|entry| {
                let sim = entry.situation.similarity(&query_sig);
                if sim >= RESONANCE_SIMILARITY_THRESHOLD {
                    Some(ResonanceMatch {
                        entry_id: entry.id.clone(),
                        similarity: sim,
                    })
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(RESONANCE_TOP_N);

        let has_prior_context = !matches.is_empty();

        ResonanceResult {
            matches,
            has_prior_context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydra_genome::ApproachSignature;

    fn make_approach() -> ApproachSignature {
        ApproachSignature::new("test", vec!["step1".into()], vec!["tool1".into()])
    }

    #[test]
    fn empty_store_no_resonance() {
        let store = GenomeStore::new();
        let result = MemoryResonance::check_resonance("deploy the api", &store);
        assert!(!result.has_prior_context);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn matching_entry_found() {
        let mut store = GenomeStore::new();
        store
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .expect("add should succeed");

        let result = MemoryResonance::check_resonance("deploy rest api service", &store);
        assert!(result.has_prior_context);
        assert!(!result.matches.is_empty());
    }

    #[test]
    fn unrelated_no_match() {
        let mut store = GenomeStore::new();
        store
            .add_from_operation("deploy rest api service", make_approach(), 0.8)
            .expect("add should succeed");

        let result = MemoryResonance::check_resonance("compile rust binary executable", &store);
        assert!(!result.has_prior_context);
    }
}
