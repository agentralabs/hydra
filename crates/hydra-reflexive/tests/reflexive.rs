//! Integration tests for hydra-reflexive.

use hydra_reflexive::{
    CapabilitySource, ModificationKind, ModificationProposal, SafeModifier, SelfModel, SelfSnapshot,
};

#[test]
fn bootstrap_and_summary() {
    let model = SelfModel::bootstrap_layer1();
    let summary = model.summary();
    assert!(summary.contains("5/5 active"));
    assert!(summary.contains("5 total ever"));
}

#[test]
fn snapshot_preserves_total_ever() {
    let model = SelfModel::bootstrap_layer1();
    let snap = SelfSnapshot::capture(&model).expect("capture");
    assert_eq!(snap.total_ever, 5);
    let restored = snap.restore().expect("restore");
    assert_eq!(restored.total_ever, 5);
}

#[test]
fn safe_modification_with_rollback() {
    let mut model = SelfModel::bootstrap_layer1();
    let mut modifier = SafeModifier::new();

    // Apply a modification
    let proposal = ModificationProposal::new(
        ModificationKind::AddCapability,
        "Add integration test capability",
        "integration-test",
    );
    modifier
        .apply(&mut model, &proposal, |m| {
            m.add_capability(
                "integration-cap",
                CapabilitySource::Sister {
                    sister_name: "test-sister".into(),
                },
            )
        })
        .expect("apply");

    assert_eq!(model.capabilities.len(), 6);

    // Rollback
    modifier.rollback_last(&mut model).expect("rollback");
    assert_eq!(model.capabilities.len(), 5);
}

#[test]
fn capability_degradation_and_restore() {
    let mut model = SelfModel::bootstrap_layer1();
    let cap = model.get_mut("hydra-constitution").expect("exists");
    cap.degrade("test degradation");
    assert!(!cap.is_active());

    let cap = model.get_mut("hydra-constitution").expect("exists");
    cap.restore();
    assert!(cap.is_active());
}

#[test]
fn active_capabilities_filters_correctly() {
    let mut model = SelfModel::bootstrap_layer1();
    assert_eq!(model.active_capabilities().len(), 5);

    let cap = model.get_mut("hydra-animus").expect("exists");
    cap.mark_unavailable("offline");

    assert_eq!(model.active_capabilities().len(), 4);
}
