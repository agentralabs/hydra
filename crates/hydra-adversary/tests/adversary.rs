//! Integration tests for hydra-adversary.

use hydra_adversary::{
    to_axiom_primitive, AntifragileStore, ImmuneAction, ImmuneSystem, ThreatActor, ThreatClass,
    ThreatEcology, ThreatSignal,
};
use hydra_axiom::AxiomPrimitive;

#[test]
fn clean_signal_passes_through() {
    let mut immune = ImmuneSystem::new();
    let signal = ThreatSignal::new(ThreatClass::Unknown, vec![], "test", "clean");
    let resp = immune.evaluate(&signal).unwrap();
    assert_eq!(resp.action, ImmuneAction::PassThrough);
}

#[test]
fn first_encounter_generates_antibody() {
    let mut immune = ImmuneSystem::new();
    let signal = ThreatSignal::new(
        ThreatClass::PromptInjection,
        vec![1.0, 0.0, 1.0],
        "attacker",
        "injection",
    );
    let resp = immune.evaluate(&signal).unwrap();
    assert_eq!(resp.action, ImmuneAction::NewAntibodyGenerated);
    assert_eq!(immune.antibody_count(), 1);
}

#[test]
fn repeat_attack_blocked() {
    let mut immune = ImmuneSystem::new();
    let features = vec![1.0, 0.0, 1.0];
    let signal1 = ThreatSignal::new(
        ThreatClass::PromptInjection,
        features.clone(),
        "attacker",
        "first",
    );
    let _ = immune.evaluate(&signal1);
    let signal2 = ThreatSignal::new(ThreatClass::PromptInjection, features, "attacker", "repeat");
    let resp = immune.evaluate(&signal2).unwrap();
    assert_eq!(resp.action, ImmuneAction::Blocked);
}

#[test]
fn resistance_only_grows() {
    let mut store = AntifragileStore::new();
    store.record_encounter(ThreatClass::PromptInjection, true);
    let r1 = store.resistance_for(&ThreatClass::PromptInjection);
    store.record_encounter(ThreatClass::PromptInjection, true);
    let r2 = store.resistance_for(&ThreatClass::PromptInjection);
    assert!(r2 > r1);
    store.record_encounter(ThreatClass::PromptInjection, false);
    let r3 = store.resistance_for(&ThreatClass::PromptInjection);
    assert!(r3 >= r2);
}

#[test]
fn constitutional_severity_is_max() {
    assert!((ThreatClass::ConstitutionalViolation.severity() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn constitutional_is_constitutional() {
    assert!(ThreatClass::ConstitutionalViolation.is_constitutional());
    assert!(ThreatClass::CausalChainManipulation.is_constitutional());
    assert!(ThreatClass::ReceiptTampering.is_constitutional());
    assert!(!ThreatClass::PromptInjection.is_constitutional());
}

#[test]
fn ecology_capabilities() {
    let mut ecology = ThreatEcology::new();
    let mut actor = ThreatActor::new("apt1", 0.9);
    actor.add_capability(ThreatClass::PromptInjection);
    ecology.add_actor(actor).unwrap();
    assert_eq!(ecology.capable_of(&ThreatClass::PromptInjection).len(), 1);
    assert_eq!(ecology.capable_of(&ThreatClass::DataExfiltration).len(), 0);
}

#[test]
fn ecology_highest_threat() {
    let mut ecology = ThreatEcology::new();
    let mut actor = ThreatActor::new("apt1", 0.9);
    actor.add_capability(ThreatClass::ConstitutionalViolation);
    ecology.add_actor(actor).unwrap();
    assert!((ecology.highest_threat() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn axiom_primitive_mapping() {
    assert_eq!(
        to_axiom_primitive(&ThreatClass::ConstitutionalViolation),
        AxiomPrimitive::AdversarialModel
    );
    assert_eq!(
        to_axiom_primitive(&ThreatClass::TrustManipulation),
        AxiomPrimitive::TrustRelation
    );
    assert_eq!(
        to_axiom_primitive(&ThreatClass::ResourceExhaustion),
        AxiomPrimitive::ResourceAllocation
    );
}

#[test]
fn constitutional_attack_triggers_immune_error() {
    let mut immune = ImmuneSystem::new();
    let signal = ThreatSignal::new(
        ThreatClass::ConstitutionalViolation,
        vec![1.0, 1.0],
        "evil",
        "constitution attack",
    );
    let result = immune.evaluate(&signal);
    assert!(result.is_err());
}
