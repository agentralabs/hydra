use hydra_belief::belief::{Belief, BeliefCategory, BeliefSource};

fn preference(subject: &str, content: &str) -> Belief {
    Belief::new(
        BeliefCategory::Preference,
        subject,
        content,
        BeliefSource::UserStated,
    )
}

// ═══════════════════════════════════════════════════════════
// SERDE TESTS
// ═══════════════════════════════════════════════════════════

#[test]
fn test_belief_serde_roundtrip() {
    let b = preference("style", "4 spaces");
    let json = serde_json::to_string(&b).unwrap();
    let de: Belief = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, b.id);
    assert_eq!(de.content, b.content);
}
