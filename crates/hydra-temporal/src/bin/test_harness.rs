//! Test harness for hydra-temporal — ~30 scenarios exercising all components.

use hydra_temporal::{
    btree::{ChronoSpatialBTree, ManifoldCoord, MemoryId, TemporalEntry},
    causal_index::CausalChainIndex,
    constants,
    constraint::{ConstraintKind, DecisionConstraint, DecisionId},
    decay::ConstraintDecay,
    decision_graph::DecisionGraph,
    query::TemporalQueryEngine,
    spatial::SpatialPartitionIndex,
    timestamp::Timestamp,
};

fn main() {
    let mut passed = 0u32;
    let mut failed = 0u32;

    macro_rules! scenario {
        ($name:expr, $body:expr) => {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
                Ok(()) => {
                    println!("  PASS: {}", $name);
                    passed += 1;
                }
                Err(_) => {
                    println!("  FAIL: {}", $name);
                    failed += 1;
                }
            }
        };
    }

    println!("=== hydra-temporal test harness ===\n");

    // ── Timestamp scenarios ──────────────────────────────────────────

    scenario!("timestamp: zero is invalid", {
        assert!(Timestamp::from_nanos(0).is_err());
    });

    scenario!("timestamp: now is valid", {
        let ts = Timestamp::now();
        assert!(ts.as_nanos() > 0);
    });

    scenario!("timestamp: nanos roundtrip", {
        let ts = Timestamp::from_nanos(42_000_000_000).unwrap();
        assert_eq!(ts.as_nanos(), 42_000_000_000);
    });

    scenario!("timestamp: datetime roundtrip", {
        let ts = Timestamp::from_nanos(1_700_000_000_123_456_789).unwrap();
        let dt = ts.to_datetime();
        let ts2 = Timestamp::from_datetime(dt).unwrap();
        assert_eq!(ts.as_nanos(), ts2.as_nanos());
    });

    scenario!("timestamp: delta_ms calculation", {
        let a = Timestamp::from_nanos(1_000_000_000).unwrap();
        let b = Timestamp::from_nanos(2_000_000_000).unwrap();
        assert_eq!(a.delta_ms(&b), 1000);
    });

    scenario!("timestamp: gaussian similarity self = 1.0", {
        let ts = Timestamp::from_nanos(1_000_000_000).unwrap();
        let sim = ts.gaussian_similarity(&ts, 1e6);
        assert!((sim - 1.0).abs() < 1e-10);
    });

    // ── B+ Tree scenarios ────────────────────────────────────────────

    scenario!("btree: insert and exact lookup", {
        let mut tree = ChronoSpatialBTree::new();
        let entry = make_entry(1000);
        tree.insert(entry).unwrap();
        assert!(tree
            .get_exact(&Timestamp::from_nanos(1000).unwrap())
            .is_some());
    });

    scenario!("btree: duplicate rejected", {
        let mut tree = ChronoSpatialBTree::new();
        tree.insert(make_entry(1000)).unwrap();
        assert!(tree.insert(make_entry(1000)).is_err());
    });

    scenario!("btree: nearest lookup", {
        let mut tree = ChronoSpatialBTree::new();
        tree.insert(make_entry(100)).unwrap();
        tree.insert(make_entry(200)).unwrap();
        tree.insert(make_entry(300)).unwrap();
        let nearest = tree
            .get_nearest(&Timestamp::from_nanos(190).unwrap())
            .unwrap();
        assert_eq!(nearest.timestamp.as_nanos(), 200);
    });

    scenario!("btree: range scan", {
        let mut tree = ChronoSpatialBTree::new();
        for i in 1..=10 {
            tree.insert(make_entry(i * 1000)).unwrap();
        }
        let from = Timestamp::from_nanos(3000).unwrap();
        let to = Timestamp::from_nanos(7000).unwrap();
        let results = tree.range_scan(&from, &to).unwrap();
        assert_eq!(results.len(), 5);
    });

    scenario!("btree: most_recent O(1)", {
        let mut tree = ChronoSpatialBTree::new();
        for i in 1..=5 {
            tree.insert(make_entry(i * 1000)).unwrap();
        }
        let recent = tree.most_recent(2);
        assert_eq!(recent[0].timestamp.as_nanos(), 5000);
    });

    // ── Latency test ─────────────────────────────────────────────────

    scenario!("btree: 10k insert + lookup < 50ms", {
        let mut tree = ChronoSpatialBTree::new();
        let start = std::time::Instant::now();
        for i in 1..=10_000u64 {
            tree.insert(make_entry(i)).unwrap();
        }
        // Do a bunch of lookups
        for i in (1..=10_000u64).step_by(100) {
            tree.get_exact(&Timestamp::from_nanos(i).unwrap());
        }
        let elapsed = start.elapsed();
        let target = std::time::Duration::from_nanos(constants::QUERY_LATENCY_TARGET_NS);
        println!("    elapsed: {:?}, target: {:?}", elapsed, target);
        assert!(elapsed < target, "latency exceeded target");
    });

    // ── Decay scenarios ──────────────────────────────────────────────

    scenario!("decay: initial strength preserved at t=0", {
        let decay = ConstraintDecay::new(0.8);
        assert!((decay.strength_at(0.0) - 0.8).abs() < 1e-10);
    });

    scenario!("decay: decays over time", {
        let decay = ConstraintDecay::new(1.0);
        assert!(decay.strength_at(1e6) < 1.0);
    });

    scenario!("decay: floor enforced", {
        let decay = ConstraintDecay::new(1.0);
        let s = decay.strength_at(1e15);
        assert!((s - constants::CONSTRAINT_DECAY_FLOOR).abs() < 1e-10);
    });

    scenario!("decay: fossil detection", {
        let decay = ConstraintDecay::new(1.0);
        assert!(!decay.is_fossil(0.0));
        assert!(decay.is_fossil(1e15));
    });

    scenario!("decay: time_to_fossil is positive", {
        let decay = ConstraintDecay::new(1.0);
        let t = decay.time_to_fossil_seconds().unwrap();
        assert!(t > 0.0);
    });

    // ── Decision Graph scenarios ─────────────────────────────────────

    scenario!("decision_graph: record and retrieve", {
        let mut graph = DecisionGraph::new();
        let c = make_decision("d1", ConstraintKind::Informational, "test", None);
        graph.record(c).unwrap();
        assert!(graph.get(&DecisionId::from_value("d1")).is_some());
    });

    scenario!("decision_graph: conflict detection", {
        let mut graph = DecisionGraph::new();
        let c = make_decision("d1", ConstraintKind::Forbids, "rm -rf", None);
        graph.record(c).unwrap();
        let conflicts = graph.check_conflicts("rm -rf /home", 0.0);
        assert!(!conflicts.is_empty());
    });

    scenario!("decision_graph: subtree traversal", {
        let mut graph = DecisionGraph::new();
        graph
            .record(make_decision(
                "root",
                ConstraintKind::Informational,
                "x",
                None,
            ))
            .unwrap();
        graph
            .record(make_decision(
                "child",
                ConstraintKind::Informational,
                "x",
                Some("root"),
            ))
            .unwrap();
        let sub = graph.subtree(&DecisionId::from_value("root")).unwrap();
        assert_eq!(sub.len(), 2);
    });

    scenario!("decision_graph: parent must exist", {
        let mut graph = DecisionGraph::new();
        let c = make_decision(
            "orphan",
            ConstraintKind::Informational,
            "x",
            Some("missing"),
        );
        assert!(graph.record(c).is_err());
    });

    // ── Spatial index scenarios ──────────────────────────────────────

    scenario!("spatial: insert and query", {
        let mut idx = SpatialPartitionIndex::new();
        let coord = ManifoldCoord::new(5.0, 5.0, 5.0);
        idx.insert(MemoryId::from_value("m1"), &coord).unwrap();
        assert_eq!(idx.memories_at(&coord).len(), 1);
    });

    scenario!("spatial: near query finds neighbors", {
        let mut idx = SpatialPartitionIndex::new();
        idx.insert(
            MemoryId::from_value("m1"),
            &ManifoldCoord::new(5.0, 5.0, 5.0),
        )
        .unwrap();
        idx.insert(
            MemoryId::from_value("m2"),
            &ManifoldCoord::new(6.0, 5.0, 5.0),
        )
        .unwrap();
        let near = idx.memories_near(&ManifoldCoord::new(5.5, 5.0, 5.0), 1);
        assert_eq!(near.len(), 2);
    });

    // ── Causal index scenarios ───────────────────────────────────────

    scenario!("causal_index: insert and lookup", {
        let mut idx = CausalChainIndex::new();
        idx.insert("root-1".to_string(), MemoryId::from_value("m1"));
        idx.insert("root-1".to_string(), MemoryId::from_value("m2"));
        assert_eq!(idx.memories_for_root("root-1").len(), 2);
    });

    scenario!("causal_index: root count", {
        let mut idx = CausalChainIndex::new();
        idx.insert("a".to_string(), MemoryId::from_value("m1"));
        idx.insert("b".to_string(), MemoryId::from_value("m2"));
        assert_eq!(idx.root_count(), 2);
    });

    // ── Query engine scenarios ───────────────────────────────────────

    scenario!("query_engine: exact timestamp via engine", {
        let mut engine = TemporalQueryEngine::new();
        let entry = make_entry(5000);
        engine.btree.insert(entry).unwrap();
        let result = engine.exact_timestamp(&Timestamp::from_nanos(5000).unwrap());
        assert!(result.is_some());
    });

    scenario!("query_engine: full chain — insert, index, query", {
        let mut engine = TemporalQueryEngine::new();
        // Insert into btree
        let entry = TemporalEntry {
            timestamp: Timestamp::from_nanos(1000).unwrap(),
            memory_id: MemoryId::from_value("chain-mem"),
            coord: ManifoldCoord::new(1.0, 2.0, 3.0),
            causal_root: Some("root-chain".to_string()),
        };
        engine.btree.insert(entry).unwrap();
        // Insert into causal index
        engine
            .causal_index
            .insert("root-chain".to_string(), MemoryId::from_value("chain-mem"));
        // Query by causal root
        let mems = engine.causal_root("root-chain");
        assert_eq!(mems.len(), 1);
        // Query by timestamp
        assert!(engine
            .exact_timestamp(&Timestamp::from_nanos(1000).unwrap())
            .is_some());
    });

    // ── Summary ──────────────────────────────────────────────────────

    println!("\n=== Results: {passed} passed, {failed} failed ===");
    if failed > 0 {
        std::process::exit(1);
    }
}

fn make_entry(nanos: u64) -> TemporalEntry {
    TemporalEntry {
        timestamp: Timestamp::from_nanos(nanos).unwrap(),
        memory_id: MemoryId::from_value(format!("mem-{nanos}")),
        coord: ManifoldCoord::new(0.0, 0.0, 0.0),
        causal_root: None,
    }
}

fn make_decision(
    id: &str,
    kind: ConstraintKind,
    pattern: &str,
    parent: Option<&str>,
) -> DecisionConstraint {
    DecisionConstraint::new(
        DecisionId::from_value(id),
        Timestamp::now(),
        kind,
        format!("Test: {id}"),
        pattern.to_string(),
        parent.map(DecisionId::from_value),
        1.0,
    )
}
