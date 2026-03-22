//! Integration tests for hydra-signals.

use hydra_animus::{
    graph::PrimeGraph,
    semiring::signal::{Signal, SignalId, SignalTier, SignalWeight},
};
use hydra_signals::{DeliveryOutcome, SignalFabric, SubscriberId};

fn make_signal(tier: SignalTier) -> Signal {
    Signal::new(
        PrimeGraph::new(),
        SignalId::identity(),
        SignalWeight::max(),
        tier,
        3,
    )
}

fn make_orphan(tier: SignalTier) -> Signal {
    let mut s = make_signal(tier);
    s.causal_chain.clear();
    s
}

#[test]
fn orphan_rejection_produces_receipt() {
    let mut fabric = SignalFabric::new();
    let orphan = make_orphan(SignalTier::Fleet);
    let signal_id = orphan.id.as_str().to_string();
    fabric.emit(orphan).unwrap();

    // Should have a rejected receipt
    let receipts = fabric.receipt_log().receipts_for(&signal_id);
    assert_eq!(receipts.len(), 1);
    assert!(matches!(
        receipts[0].outcome,
        DeliveryOutcome::Rejected { .. }
    ));
}

#[test]
fn priority_preservation_across_tiers() {
    let mut fabric = SignalFabric::new();

    // Subscribe to all tiers we'll emit
    for topic in &[
        "signal.prediction",
        "signal.fleet",
        "signal.adversarial",
        "signal.belief_revision",
    ] {
        fabric
            .subscribe(
                topic,
                SubscriberId::from_value(&format!("sub-{}", topic)),
                &format!("{}-handler", topic),
            )
            .unwrap();
    }

    // Emit in reverse priority order
    fabric.emit(make_signal(SignalTier::Prediction)).unwrap();
    fabric.emit(make_signal(SignalTier::Fleet)).unwrap();

    // Adversarial is immediate — not queued
    fabric.emit(make_signal(SignalTier::Adversarial)).unwrap();

    fabric
        .emit(make_signal(SignalTier::BeliefRevision))
        .unwrap();

    // Only fleet, belief_revision, and prediction are queued (adversarial is immediate)
    assert_eq!(fabric.status().queued, 3);

    let result = fabric.dispatch();
    assert_eq!(result.dispatched, 3);
    assert_eq!(fabric.status().queued, 0);
}

#[test]
fn receipt_accumulation_across_operations() {
    let mut fabric = SignalFabric::new();

    // Constitutional signal — immediate receipt
    fabric.emit(make_signal(SignalTier::Constitution)).unwrap();
    assert!(fabric.receipt_log().len() >= 1);

    // Orphan signal — rejection receipt
    fabric.emit(make_orphan(SignalTier::Fleet)).unwrap();
    assert!(fabric.receipt_log().len() >= 2);

    // Fleet signal — queued, then dispatched
    fabric.emit(make_signal(SignalTier::Fleet)).unwrap();
    fabric.dispatch();
    // Fleet with no subscriber produces a dropped receipt
    assert!(fabric.receipt_log().len() >= 3);
}

#[test]
fn constitutional_bypass_with_audit_trail() {
    let mut fabric = SignalFabric::new();

    fabric.emit(make_signal(SignalTier::Constitution)).unwrap();

    // Not queued
    assert_eq!(fabric.status().queued, 0);
    // Receipt created
    assert!(fabric.receipt_log().len() >= 1);
    assert_eq!(fabric.receipt_log().success_count(), 1);
    // Audit trail has entries (gate passed, routed, dispatched)
    assert!(fabric.audit_trail().len() >= 3);
}
