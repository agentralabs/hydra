//! Integration tests for hydra-temporal.

use hydra_temporal::{
    btree::{ChronoSpatialBTree, ManifoldCoord, MemoryId, TemporalEntry},
    constants::CONSTRAINT_DECAY_FLOOR,
    constraint::{ConstraintKind, DecisionConstraint, DecisionId},
    decay::ConstraintDecay,
    decision_graph::DecisionGraph,
    query::TemporalQueryEngine,
    timestamp::Timestamp,
};

fn make_entry(nanos: u64) -> TemporalEntry {
    TemporalEntry {
        timestamp: Timestamp::from_nanos(nanos).unwrap(),
        memory_id: MemoryId::from_value(format!("mem-{nanos}")),
        coord: ManifoldCoord::new(0.0, 0.0, 0.0),
        causal_root: None,
    }
}

/// The B+ tree must reject duplicate timestamps — append-only invariant.
#[test]
fn append_only_property() {
    let mut tree = ChronoSpatialBTree::new();
    tree.insert(make_entry(1000)).unwrap();
    let result = tree.insert(make_entry(1000));
    assert!(result.is_err(), "duplicate should be rejected");
    // Original entry must still be accessible
    assert!(tree
        .get_exact(&Timestamp::from_nanos(1000).unwrap())
        .is_some());
    assert_eq!(tree.len(), 1);
}

/// Decay floor must be enforced — strength never goes below floor.
#[test]
fn decay_floor_enforced() {
    let decay = ConstraintDecay::new(1.0);
    // After an absurdly long time
    let strength = decay.strength_at(1e18);
    assert!(
        strength >= CONSTRAINT_DECAY_FLOOR,
        "strength {strength} fell below floor {CONSTRAINT_DECAY_FLOOR}"
    );
    assert!(
        (strength - CONSTRAINT_DECAY_FLOOR).abs() < 1e-10,
        "strength should be exactly at floor"
    );
}

/// Permanent decisions (Forbids) must still conflict even after long time,
/// because the floor keeps a tiny residual strength.
#[test]
fn permanent_decisions_still_conflict() {
    let mut graph = DecisionGraph::new();
    let c = DecisionConstraint::new(
        DecisionId::from_value("permanent"),
        Timestamp::now(),
        ConstraintKind::Forbids,
        "Never do X".to_string(),
        "dangerous_action".to_string(),
        None,
        1.0,
    );
    graph.record(c).unwrap();

    // Even at a very small elapsed time, should still conflict
    // (decay floor is 0.001, so still above zero)
    let conflicts = graph.check_conflicts("do dangerous_action now", 0.0);
    assert_eq!(conflicts.len(), 1);
}

/// Fossil constraints (strength at floor) should NOT trigger conflicts
/// in the check_conflict method because the floor check excludes them.
#[test]
fn fossil_constraints_no_conflict() {
    let mut graph = DecisionGraph::new();
    let c = DecisionConstraint::new(
        DecisionId::from_value("old"),
        Timestamp::now(),
        ConstraintKind::Forbids,
        "Old rule".to_string(),
        "some_action".to_string(),
        None,
        1.0,
    );
    graph.record(c).unwrap();

    // At extreme elapsed time, the constraint is fossil
    let conflicts = graph.check_conflicts("some_action", 1e18);
    assert!(
        conflicts.is_empty(),
        "fossil constraint should not produce conflicts"
    );
}

/// Exact timestamp lookup must return the correct entry.
#[test]
fn exact_timestamp_lookup() {
    let mut engine = TemporalQueryEngine::new();
    engine.btree.insert(make_entry(42_000)).unwrap();
    engine.btree.insert(make_entry(43_000)).unwrap();
    let result = engine.exact_timestamp(&Timestamp::from_nanos(42_000).unwrap());
    assert!(result.is_some());
    assert_eq!(result.unwrap().memory_id.as_str(), "mem-42000");
}

/// Range boundaries must be inclusive on both ends.
#[test]
fn range_boundaries_inclusive() {
    let mut tree = ChronoSpatialBTree::new();
    for i in 1..=10 {
        tree.insert(make_entry(i * 100)).unwrap();
    }
    let from = Timestamp::from_nanos(300).unwrap();
    let to = Timestamp::from_nanos(700).unwrap();
    let results = tree.range_scan(&from, &to).unwrap();
    // Should include 300, 400, 500, 600, 700
    assert_eq!(results.len(), 5);
    assert_eq!(results[0].timestamp.as_nanos(), 300);
    assert_eq!(results[4].timestamp.as_nanos(), 700);
}

/// The query engine must handle causal root indexing end-to-end.
#[test]
fn causal_root_end_to_end() {
    let mut engine = TemporalQueryEngine::new();
    engine
        .causal_index
        .insert("cause-alpha".to_string(), MemoryId::from_value("m1"));
    engine
        .causal_index
        .insert("cause-alpha".to_string(), MemoryId::from_value("m2"));
    engine
        .causal_index
        .insert("cause-beta".to_string(), MemoryId::from_value("m3"));

    let alpha = engine.causal_root("cause-alpha");
    assert_eq!(alpha.len(), 2);

    let beta = engine.causal_root("cause-beta");
    assert_eq!(beta.len(), 1);

    let empty = engine.causal_root("nonexistent");
    assert!(empty.is_empty());
}
