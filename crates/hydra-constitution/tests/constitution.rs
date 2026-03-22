//! Integration tests for hydra-constitution.
//! Tests the full checker end to end.

use hydra_constitution::{
    constants::CONSTITUTIONAL_IDENTITY_ID, laws::LawCheckContext, ConstitutionChecker,
};

fn root() -> String {
    CONSTITUTIONAL_IDENTITY_ID.to_string()
}
fn checker() -> ConstitutionChecker {
    ConstitutionChecker::new()
}

fn clean(action: &str) -> LawCheckContext {
    LawCheckContext::new("integration-test", action).with_causal_chain(vec![root()])
}

#[test]
fn checker_has_all_seven_laws() {
    assert_eq!(checker().law_count(), 7);
}

#[test]
fn all_laws_checked_on_every_call() {
    let result = checker().check(&clean("agent.spawn"));
    assert_eq!(result.laws_checked.len(), 7);
}

#[test]
fn clean_action_passes_all_laws() {
    assert!(checker().check(&clean("agent.spawn")).is_permitted());
    assert!(checker().check(&clean("receipt.write")).is_permitted());
    assert!(checker().check(&clean("memory.read")).is_permitted());
    assert!(checker().check(&clean("signal.emit")).is_permitted());
}

#[test]
fn every_mutation_action_is_blocked() {
    let mutations = [
        "receipt.delete",
        "receipt.modify",
        "receipt.suppress",
        "receipt.overwrite",
        "receipt.purge",
        "receipt.clear",
    ];
    for action in &mutations {
        let ctx = LawCheckContext::new("test", *action).with_causal_chain(vec![root()]);
        assert!(
            !checker().check(&ctx).is_permitted(),
            "Expected {} to be blocked",
            action
        );
    }
}

#[test]
fn orphan_action_is_always_blocked() {
    let ctx = LawCheckContext::new("orphan", "agent.spawn").with_causal_chain(vec![]);
    assert!(!checker().check(&ctx).is_permitted());
}

#[test]
fn constitution_modification_is_always_blocked() {
    let attempts = [
        "constitution.modify",
        "constitution.patch",
        "constitution.bypass",
        "constitution.override",
    ];
    for action in &attempts {
        let ctx = LawCheckContext::new("test", *action).with_causal_chain(vec![root()]);
        assert!(
            !checker().check(&ctx).is_permitted(),
            "Expected {} to be blocked",
            action
        );
    }
}

#[test]
fn reserved_identity_impersonation_is_blocked() {
    let reserved = ["hydra", "hydra-kernel", "hydra-constitution"];
    for identity in &reserved {
        let ctx = LawCheckContext::new("test", "identity.claim")
            .with_meta("claiming_identity", *identity)
            .with_causal_chain(vec![root()]);
        assert!(
            !checker().check(&ctx).is_permitted(),
            "Expected claiming '{}' to be blocked",
            identity
        );
    }
}
