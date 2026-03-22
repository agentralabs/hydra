//! Integration tests for hydra-memory.
//! Tests that don't need a live AgenticMemory file.

use hydra_memory::{
    identity::IdentityProfile,
    layers::{MemoryLayer, MemoryRecord},
    session::SessionRecord,
    temporal_bridge::TemporalBridge,
    verbatim::{ContextSnapshot, Surface, VerbatimRecord},
};
use hydra_temporal::{btree::ManifoldCoord, timestamp::Timestamp};

fn ts(n: u64) -> Timestamp {
    Timestamp::from_nanos(n).expect("valid nanos")
}

#[test]
fn verbatim_write_ahead_and_finalize_cycle() {
    let mut record = VerbatimRecord::begin(
        "test-session",
        0,
        Surface::Tui,
        "what is the status of the deployment?",
        ContextSnapshot::default(),
        "const-identity",
    )
    .expect("should create record");

    // Before response: no hash
    assert!(record.content_hash.is_none());

    // After response: hash computed
    record.finalize("Deployment is healthy -- 3 pods running", 0.05);
    assert!(record.content_hash.is_some());
    assert!(record.verify_integrity().is_ok());
}

#[test]
fn temporal_bridge_monotonically_increasing() {
    let mut bridge = TemporalBridge::new();
    let before = bridge.total_indexed();

    for i in 1..=10u64 {
        bridge
            .index(
                &format!("mem-{}", i),
                ts(i * 1_000_000_000),
                ManifoldCoord::new(0.0, 0.0, 0.0),
                "const-identity",
                "session",
            )
            .expect("should index");
    }

    assert!(bridge.total_indexed() >= before + 10);
}

#[test]
fn identity_profile_sessions_never_decrease() {
    let mut profile = IdentityProfile::new();
    for _i in 0..20 {
        let before = profile.sessions_observed;
        profile.observe_session(30.0, 10);
        assert!(profile.sessions_observed >= before);
    }
    assert_eq!(profile.sessions_observed, 20);
}

#[test]
fn all_memory_layers_produce_valid_content() {
    let layers = [
        MemoryLayer::Verbatim,
        MemoryLayer::Episodic,
        MemoryLayer::Semantic,
        MemoryLayer::Relational,
        MemoryLayer::Causal,
        MemoryLayer::Procedural,
        MemoryLayer::Anticipatory,
        MemoryLayer::Identity,
    ];
    for layer in &layers {
        let record = MemoryRecord::new(
            layer.clone(),
            serde_json::json!({"test": "data"}),
            "test-session",
            "const-identity",
        );
        let content = record.to_cognitive_content();
        assert!(
            content.contains(layer.tag()),
            "Layer {:?} tag missing",
            layer
        );
        assert!(!content.is_empty());
    }
}

#[test]
fn session_exchange_count_monotonic() {
    let mut session = SessionRecord::new();
    for _i in 0..50 {
        let before = session.exchange_count;
        session.record_exchange();
        assert!(session.exchange_count >= before);
    }
    assert_eq!(session.exchange_count, 50);
}
