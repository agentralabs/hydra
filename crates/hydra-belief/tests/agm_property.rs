//! Property-based tests for AGM postulates using proptest.
//!
//! Build Law 7: "AGM postulates are VERIFIED BY PROPERTY TESTS."
//! These tests verify that the three core AGM postulates hold
//! for arbitrary beliefs, not just hand-picked examples.

use hydra_belief::{
    revise, verify_consistency, verify_inclusion, verify_success, Belief, BeliefCategory,
    BeliefStore, RevisionPolicy,
};
use proptest::prelude::*;

/// Strategy to generate a valid confidence value in [0.0, 1.0].
fn confidence_strategy() -> impl Strategy<Value = f64> {
    (0u32..=100).prop_map(|n| f64::from(n) / 100.0)
}

/// Strategy to generate a proposition string with at least one word > 3 chars.
fn proposition_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            "deployment",
            "system",
            "memory",
            "performance",
            "security",
            "network",
            "storage",
            "analysis",
            "processing",
            "monitoring",
        ]),
        1..=4,
    )
    .prop_map(|words| words.join(" "))
}

/// Strategy to generate a revision policy.
fn policy_strategy() -> impl Strategy<Value = RevisionPolicy> {
    prop::sample::select(vec![
        RevisionPolicy::Standard,
        RevisionPolicy::Protected,
        RevisionPolicy::Immutable,
    ])
}

/// Strategy to generate a belief category.
fn category_strategy() -> impl Strategy<Value = BeliefCategory> {
    prop::sample::select(vec![
        BeliefCategory::World,
        BeliefCategory::Capability,
        BeliefCategory::Trust,
        BeliefCategory::Temporal,
        BeliefCategory::Security,
    ])
}

/// Strategy to generate a complete Belief.
fn belief_strategy() -> impl Strategy<Value = Belief> {
    (
        proposition_strategy(),
        confidence_strategy(),
        category_strategy(),
        policy_strategy(),
    )
        .prop_map(|(prop, conf, cat, pol)| Belief::new(prop, conf, cat, pol))
}

proptest! {
    /// AGM Success Postulate (property test):
    /// After revising a belief set K with a new belief phi,
    /// phi MUST be present in the resulting belief set.
    /// This must hold for ALL arbitrary beliefs.
    #[test]
    fn agm_success_postulate(
        initial in prop::collection::vec(belief_strategy(), 0..5),
        new_belief in belief_strategy(),
    ) {
        let mut store = BeliefStore::new();
        for b in initial {
            let _ = store.insert(b);
        }

        let result = revise(&mut store, new_belief);
        match result {
            Ok(outcome) => {
                prop_assert!(
                    verify_success(&store, &outcome.belief_id).is_ok(),
                    "AGM success: new belief must exist in store after revision"
                );
            }
            Err(e) => {
                // Only acceptable error is store full
                prop_assert!(
                    format!("{e}").contains("full"),
                    "unexpected revision error: {e}"
                );
            }
        }
    }

    /// AGM Inclusion Postulate (property test):
    /// After revising K with phi, no original beliefs are lost.
    /// The revised set contains all original beliefs (possibly modified)
    /// plus the new belief.
    #[test]
    fn agm_inclusion_postulate(
        initial in prop::collection::vec(belief_strategy(), 1..5),
        new_belief in belief_strategy(),
    ) {
        let mut store = BeliefStore::new();
        let mut original_ids = Vec::new();
        for b in initial {
            let id = b.id.clone();
            if store.insert(b).is_ok() {
                original_ids.push(id);
            }
        }

        let result = revise(&mut store, new_belief);
        match result {
            Ok(_outcome) => {
                prop_assert!(
                    verify_inclusion(&original_ids, &store).is_ok(),
                    "AGM inclusion: no original beliefs should be lost"
                );
            }
            Err(e) => {
                prop_assert!(
                    format!("{e}").contains("full"),
                    "unexpected revision error: {e}"
                );
            }
        }
    }

    /// AGM Consistency Postulate (property test):
    /// After revising K with phi, if phi is consistent with K,
    /// the resulting belief set should not contain direct contradictions.
    /// Our simplified check: no two high-confidence beliefs directly
    /// contradict each other.
    #[test]
    fn agm_consistency_postulate(
        initial in prop::collection::vec(belief_strategy(), 0..5),
        new_belief in belief_strategy(),
    ) {
        let mut store = BeliefStore::new();
        for b in initial {
            let _ = store.insert(b);
        }

        let result = revise(&mut store, new_belief);
        if result.is_ok() {
            // Consistency check should pass after any revision
            prop_assert!(
                verify_consistency(&store).is_ok(),
                "AGM consistency: no contradictions after revision"
            );
        }
    }

    /// Protected beliefs never decrease in confidence (property test).
    /// This is the capability protection invariant:
    /// Protected (capability) beliefs can only increase, never decrease.
    #[test]
    fn protected_beliefs_never_decrease(
        cap_proposition in proposition_strategy(),
        cap_confidence in confidence_strategy(),
        new_proposition in proposition_strategy(),
        new_confidence in confidence_strategy(),
    ) {
        let mut store = BeliefStore::new();
        let cap = Belief::new(
            cap_proposition,
            cap_confidence,
            BeliefCategory::Capability,
            RevisionPolicy::Protected,
        );
        let cap_id = cap.id.clone();
        let original_confidence = cap.confidence;
        store.insert(cap).unwrap();

        let new_b = Belief::new(
            new_proposition,
            new_confidence,
            BeliefCategory::World,
            RevisionPolicy::Standard,
        );
        let _ = revise(&mut store, new_b);

        if let Some(cap_after) = store.get(&cap_id) {
            prop_assert!(
                cap_after.confidence >= original_confidence - f64::EPSILON,
                "protected belief confidence must not decrease: was {}, now {}",
                original_confidence,
                cap_after.confidence
            );
        }
    }

    /// Immutable beliefs are never modified (property test).
    /// Immutable beliefs must retain their exact confidence after any revision.
    #[test]
    fn immutable_beliefs_unchanged(
        imm_proposition in proposition_strategy(),
        imm_confidence in confidence_strategy(),
        new_proposition in proposition_strategy(),
        new_confidence in confidence_strategy(),
    ) {
        let mut store = BeliefStore::new();
        let imm = Belief::new(
            imm_proposition,
            imm_confidence,
            BeliefCategory::World,
            RevisionPolicy::Immutable,
        );
        let imm_id = imm.id.clone();
        let original_confidence = imm.confidence;
        store.insert(imm).unwrap();

        let new_b = Belief::world(new_proposition, new_confidence);
        let _ = revise(&mut store, new_b);

        if let Some(imm_after) = store.get(&imm_id) {
            prop_assert!(
                (imm_after.confidence - original_confidence).abs() < f64::EPSILON,
                "immutable belief must not change: was {}, now {}",
                original_confidence,
                imm_after.confidence
            );
        }
    }
}
