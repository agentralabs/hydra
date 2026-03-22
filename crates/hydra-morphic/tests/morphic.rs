//! Integration tests for hydra-morphic.

use hydra_morphic::{MorphicEventKind, MorphicIdentity, MorphicSignature};

#[test]
fn signature_deepening_is_monotonic() {
    let mut sig = MorphicSignature::genesis();
    for i in 0..10 {
        let depth_before = sig.depth;
        sig.deepen(&format!("event-{i}"));
        assert!(sig.depth > depth_before);
    }
}

#[test]
fn identity_diverges_after_events() {
    let mut a = MorphicIdentity::genesis();
    let b = a.clone();

    // Record many events to push distance above threshold
    for i in 0..20 {
        a.record_event(MorphicEventKind::CapabilityAdded {
            name: format!("cap-{i}"),
        })
        .expect("record");
    }

    let distance = a.signature.distance(&b.signature);
    assert!(distance > 0.0);
}

#[test]
fn restart_tracking() {
    let mut identity = MorphicIdentity::genesis();
    identity.record_restart().expect("restart");
    assert!(identity.signature.restart_count > 0);
    assert!(!identity.history.is_empty());
}

#[test]
fn distance_symmetry() {
    let mut a = MorphicIdentity::genesis();
    let mut b = MorphicIdentity::genesis();

    a.record_event(MorphicEventKind::SkillLoaded {
        skill_id: "s1".into(),
    })
    .expect("record");
    b.record_event(MorphicEventKind::SkillLoaded {
        skill_id: "s2".into(),
    })
    .expect("record");

    let d1 = a.signature.distance(&b.signature);
    let d2 = b.signature.distance(&a.signature);
    assert!((d1 - d2).abs() < 1e-10);
}
