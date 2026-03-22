//! Integration tests for hydra-persona.

use hydra_persona::{Persona, PersonaBlend, PersonaRegistry};

#[test]
fn core_persona_preloaded_in_registry() {
    let registry = PersonaRegistry::new();
    assert!(registry.get("hydra-core").is_some());
}

#[test]
fn register_and_blend_two_personas() {
    let mut registry = PersonaRegistry::new();
    registry
        .register(Persona::security_analyst_persona())
        .expect("register");

    let blend = PersonaBlend::weighted(vec![
        ("hydra-core".into(), 0.7),
        ("security-analyst".into(), 0.3),
    ])
    .expect("blend");

    registry.set_blend(blend).expect("set_blend");
    let voice = registry.active_voice().expect("voice");
    assert!(voice.is_active());
    assert!(voice.summary().contains("hydra-core"));
}

#[test]
fn invalid_weights_rejected() {
    let result = PersonaBlend::weighted(vec![("a".into(), 0.5), ("b".into(), 0.6)]);
    assert!(result.is_err());
}

#[test]
fn persona_builders_work() {
    let p = Persona::new("test", "test persona")
        .with_vocabulary(vec!["word1".into()])
        .with_priorities(vec!["p1".into()])
        .with_tone("friendly");
    assert_eq!(p.name, "test");
    assert_eq!(p.vocabulary.len(), 1);
    assert_eq!(p.priorities.len(), 1);
    assert_eq!(p.tone, "friendly");
}

#[test]
fn all_builtin_personas_have_content() {
    let core = Persona::core_persona();
    assert!(!core.vocabulary.is_empty());
    assert!(!core.priorities.is_empty());

    let sec = Persona::security_analyst_persona();
    assert!(!sec.vocabulary.is_empty());

    let arch = Persona::software_architect_persona();
    assert!(!arch.vocabulary.is_empty());
}
