//! AGM belief revision implementation.

use crate::belief::{Belief, RevisionPolicy};
use crate::constants::REVISION_STRENGTH;
use crate::errors::BeliefError;
use crate::manifold::BeliefPosition;
use crate::store::BeliefStore;
use hydra_constitution::{ConstitutionChecker, LawCheckContext};

/// Result of a belief revision operation.
#[derive(Debug, Clone)]
pub struct RevisionResult {
    /// The ID of the newly installed or revised belief.
    pub belief_id: String,
    /// IDs of beliefs that were revised as a side effect.
    pub revised_ids: Vec<String>,
    /// Number of contradictions resolved.
    pub contradictions_resolved: usize,
}

/// Perform AGM belief revision: install a new belief and resolve contradictions.
///
/// Steps:
/// 1. Install the new belief into the store.
/// 2. Find beliefs that contradict the new one (proposition overlap).
/// 3. Revise contradicting beliefs via geodesic flow.
/// 4. Propagate confidence changes.
pub fn revise(store: &mut BeliefStore, new_belief: Belief) -> Result<RevisionResult, BeliefError> {
    // Constitutional check before revision
    let checker = ConstitutionChecker::new();
    let ctx = LawCheckContext::new(&new_belief.id, "belief.revise")
        .with_meta("proposition", &new_belief.proposition)
        .with_meta("confidence", new_belief.confidence.to_string())
        .with_meta("provenance_source", "agm-revision")
        .with_meta("revision_cause", &new_belief.proposition);
    if let Err(e) = checker.check_strict(&ctx) {
        eprintln!("hydra: belief revision BLOCKED by constitution: {e}");
        return Err(BeliefError::RevisionDenied {
            reason: format!("constitutional violation: {e}"),
        });
    }

    let new_id = new_belief.id.clone();
    let new_proposition = new_belief.proposition.clone();
    let new_confidence = new_belief.confidence;

    // Step 1: Install the new belief
    store.insert(new_belief)?;

    // Step 2: Find contradicting beliefs
    let all_ids: Vec<String> = store
        .all()
        .iter()
        .filter(|b| b.id != new_id && proposition_overlap(&b.proposition, &new_proposition))
        .map(|b| b.id.clone())
        .collect();

    // Step 3 & 4: Revise contradicting beliefs via geodesic
    let mut revised_ids = Vec::new();
    for id in &all_ids {
        if let Some(existing) = store.get_mut(id) {
            if !existing.is_revisable() {
                continue;
            }

            let current_pos = BeliefPosition::new(vec![existing.confidence]);
            let target_pos = BeliefPosition::new(vec![new_confidence]);
            let stepped = current_pos.geodesic_step(&target_pos);

            let delta = stepped.coordinates.first().copied().unwrap_or(0.0) - existing.confidence;
            let scaled_delta = delta * REVISION_STRENGTH;

            // Protected beliefs cannot decrease
            if matches!(existing.policy, RevisionPolicy::Protected) && scaled_delta < 0.0 {
                continue;
            }

            existing.apply_delta(scaled_delta);
            revised_ids.push(id.clone());
        }
    }

    let contradictions_resolved = revised_ids.len();

    Ok(RevisionResult {
        belief_id: new_id,
        revised_ids,
        contradictions_resolved,
    })
}

/// Check if two propositions overlap (share significant words).
///
/// Two propositions overlap if they share at least one meaningful word
/// (length > 3 to skip articles and prepositions).
pub fn proposition_overlap(a: &str, b: &str) -> bool {
    let words_a: Vec<&str> = a.split_whitespace().filter(|w| w.len() > 3).collect();
    let words_b: Vec<&str> = b.split_whitespace().filter(|w| w.len() > 3).collect();

    words_a.iter().any(|wa| words_b.contains(wa))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::Belief;

    #[test]
    fn proposition_overlap_detects_shared_words() {
        assert!(proposition_overlap(
            "the weather is sunny",
            "weather will change"
        ));
        assert!(!proposition_overlap("the cat sat", "dogs run fast"));
    }

    #[test]
    fn revise_installs_new_belief() {
        let mut store = BeliefStore::new();
        let b = Belief::world("the sky is blue", 0.9);
        let result = revise(&mut store, b).unwrap();
        assert!(store.get(&result.belief_id).is_some());
    }

    #[test]
    fn revise_resolves_contradictions() {
        let mut store = BeliefStore::new();
        let b1 = Belief::world("deployment is risky", 0.8);
        store.insert(b1).unwrap();

        let b2 = Belief::world("deployment is safe", 0.9);
        let result = revise(&mut store, b2).unwrap();
        // "deployment" overlaps so b1 should be revised
        assert_eq!(result.contradictions_resolved, 1);
    }

    #[test]
    fn protected_belief_not_decreased_by_revision() {
        let mut store = BeliefStore::new();
        let cap = Belief::capability("coding skill is high", 0.9);
        let cap_id = cap.id.clone();
        store.insert(cap).unwrap();

        let new_b = Belief::world("coding skill is low", 0.2);
        revise(&mut store, new_b).unwrap();

        let cap_after = store.get(&cap_id).unwrap();
        assert!(
            cap_after.confidence >= 0.9 - f64::EPSILON,
            "protected capability belief must not decrease"
        );
    }
}
