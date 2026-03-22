//! AGM postulate verification.

use crate::belief::Belief;
use crate::errors::BeliefError;
use crate::store::BeliefStore;

/// Verify the AGM success postulate: after revision with phi, phi is in the result.
pub fn verify_success(store: &BeliefStore, belief_id: &str) -> Result<(), BeliefError> {
    if store.get(belief_id).is_some() {
        Ok(())
    } else {
        Err(BeliefError::AgmPostulateViolation {
            postulate: "success".to_string(),
            reason: format!("belief '{belief_id}' not found after revision"),
        })
    }
}

/// Verify the AGM inclusion postulate: the revised set is a subset of
/// the expansion of the original set with the new belief.
///
/// Simplified check: all original beliefs still exist (possibly modified).
pub fn verify_inclusion(original_ids: &[String], store: &BeliefStore) -> Result<(), BeliefError> {
    for id in original_ids {
        if store.get(id).is_none() {
            return Err(BeliefError::AgmPostulateViolation {
                postulate: "inclusion".to_string(),
                reason: format!("original belief '{id}' was lost during revision"),
            });
        }
    }
    Ok(())
}

/// Verify the AGM consistency postulate: no two beliefs in the store
/// directly contradict each other with full confidence.
///
/// Simplified: if two beliefs share a proposition keyword and both have
/// confidence > 0.9, they should not have opposing sentiment.
pub fn verify_consistency(store: &BeliefStore) -> Result<(), BeliefError> {
    let beliefs = store.all();
    for (i, a) in beliefs.iter().enumerate() {
        for b in beliefs.iter().skip(i + 1) {
            if is_direct_contradiction(a, b) {
                return Err(BeliefError::AgmPostulateViolation {
                    postulate: "consistency".to_string(),
                    reason: format!(
                        "direct contradiction between '{}' and '{}'",
                        a.proposition, b.proposition
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Check if two high-confidence beliefs directly contradict each other.
fn is_direct_contradiction(a: &Belief, b: &Belief) -> bool {
    if a.confidence < 0.9 || b.confidence < 0.9 {
        return false;
    }

    let a_words: Vec<&str> = a.proposition.split_whitespace().collect();
    let b_words: Vec<&str> = b.proposition.split_whitespace().collect();

    // Check for "not" negation pattern
    let a_has_not = a_words.contains(&"not");
    let b_has_not = b_words.contains(&"not");

    if a_has_not == b_has_not {
        return false;
    }

    // Check if the non-negation words overlap significantly
    let a_content: Vec<&&str> = a_words
        .iter()
        .filter(|w| **w != "not" && w.len() > 3)
        .collect();
    let b_content: Vec<&&str> = b_words
        .iter()
        .filter(|w| **w != "not" && w.len() > 3)
        .collect();

    let overlap = a_content.iter().filter(|w| b_content.contains(w)).count();

    overlap >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::Belief;

    #[test]
    fn success_postulate_holds() {
        let mut store = BeliefStore::new();
        let b = Belief::world("test", 0.5);
        let id = b.id.clone();
        store.insert(b).unwrap();
        assert!(verify_success(&store, &id).is_ok());
    }

    #[test]
    fn success_postulate_fails_for_missing() {
        let store = BeliefStore::new();
        assert!(verify_success(&store, "nonexistent").is_err());
    }

    #[test]
    fn inclusion_postulate_holds() {
        let mut store = BeliefStore::new();
        let b1 = Belief::world("a", 0.5);
        let b2 = Belief::world("b", 0.6);
        let ids = vec![b1.id.clone(), b2.id.clone()];
        store.insert(b1).unwrap();
        store.insert(b2).unwrap();
        assert!(verify_inclusion(&ids, &store).is_ok());
    }

    #[test]
    fn consistency_postulate_no_contradiction() {
        let mut store = BeliefStore::new();
        store
            .insert(Belief::world("the sky is blue", 0.95))
            .unwrap();
        store.insert(Belief::world("grass is green", 0.95)).unwrap();
        assert!(verify_consistency(&store).is_ok());
    }
}
