//! Integration tests for hydra-belief.

use hydra_belief::{
    revise, verify_consistency, verify_inclusion, verify_success, Belief, BeliefStore,
};

#[test]
fn full_revision_cycle() {
    let mut store = BeliefStore::new();

    // Install initial beliefs
    let b1 = Belief::world("deployment risk is high", 0.8);
    let b1_id = b1.id.clone();
    store.insert(b1).unwrap();

    let b2 = Belief::capability("coding ability is strong", 0.85);
    let b2_id = b2.id.clone();
    store.insert(b2).unwrap();

    let original_ids = vec![b1_id.clone(), b2_id.clone()];

    // Revise with contradicting belief
    let new_b = Belief::world("deployment risk is low", 0.9);
    let result = revise(&mut store, new_b).unwrap();

    // AGM postulates
    verify_success(&store, &result.belief_id).unwrap();
    verify_inclusion(&original_ids, &store).unwrap();

    // Capability belief must not have decreased
    let cap = store.get(&b2_id).unwrap();
    assert!(cap.confidence >= 0.85 - f64::EPSILON);
}

#[test]
fn capability_beliefs_survive_revision() {
    let mut store = BeliefStore::new();
    let cap = Belief::capability("I can analyze code well", 0.9);
    let cap_id = cap.id.clone();
    store.insert(cap).unwrap();

    // Try to revise with a low-confidence contradicting world belief
    let low = Belief::world("code analysis ability is poor", 0.95);
    revise(&mut store, low).unwrap();

    let cap_after = store.get(&cap_id).unwrap();
    assert!(
        cap_after.confidence >= 0.9 - f64::EPSILON,
        "capability belief confidence must not decrease"
    );
}

#[test]
fn consistency_after_multiple_revisions() {
    let mut store = BeliefStore::new();
    store
        .insert(Belief::world("the system is stable", 0.7))
        .unwrap();
    store
        .insert(Belief::world("memory usage is low", 0.6))
        .unwrap();

    let new_b = Belief::world("the system needs optimization", 0.8);
    revise(&mut store, new_b).unwrap();

    // Should not have direct contradictions
    verify_consistency(&store).unwrap();
}
