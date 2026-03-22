//! Integration tests for hydra-animus.

use hydra_animus::*;

#[test]
fn full_pipeline_text_to_graph_to_signal_to_binary() {
    // Step 1: Text -> Signal
    let resolved = text_to_signal("deploy staging", 2, SignalId::identity()).unwrap();
    assert_eq!(resolved.graph.node_count(), 2);
    assert!(resolved.signal.chain_is_complete());

    // Step 2: Signal -> Binary
    let bytes = serialize_signal(&resolved.signal).unwrap();
    assert!(bytes.len() > 12); // at least header

    // Step 3: Binary -> Signal
    let restored = deserialize_signal(&bytes).unwrap();
    assert_eq!(restored.id, resolved.signal.id);
    assert_eq!(restored.tier, SignalTier::Fleet);

    // Step 4: Graph -> Text
    let readable = graph_to_text(&resolved.graph).unwrap();
    assert!(readable.summary.contains("deploy staging"));
}

#[test]
fn compose_and_merge_produce_valid_signals() {
    let a = Signal::new(
        PrimeGraph::new(),
        SignalId::identity(),
        SignalWeight::max(),
        SignalTier::Fleet,
        3,
    );
    let b = Signal::new(
        PrimeGraph::new(),
        SignalId::identity(),
        SignalWeight::max(),
        SignalTier::Companion,
        5,
    );

    // Compose
    let composed = compose(&a, &b).unwrap();
    assert!(composed.chain_is_complete());
    assert!(validate_chain(&composed).is_ok());
    assert!(validate_for_bus(&composed).is_ok());

    // Merge
    let merged = merge(&a, &b).unwrap();
    assert!(merged.chain_is_complete());
    assert!(validate_chain(&merged).is_ok());
    assert!(validate_for_bus(&merged).is_ok());
}

#[test]
fn orphan_signal_rejected_everywhere() {
    let mut orphan = Signal::new(
        PrimeGraph::new(),
        SignalId::identity(),
        SignalWeight::max(),
        SignalTier::Fleet,
        3,
    );
    orphan.causal_chain.clear();

    assert!(is_orphan(&orphan));
    assert!(validate_chain(&orphan).is_err());
    assert!(validate_for_bus(&orphan).is_err());
    assert!(matches!(
        bus::router::route(&orphan),
        RoutingDecision::Drop { .. }
    ));
}

#[test]
fn graph_serialization_round_trip() {
    let mut g = PrimeGraph::new();
    let a = g
        .add_node(Node::new(
            NodeType::Belief,
            serde_json::json!({"fact": "test"}),
        ))
        .unwrap();
    let b = g
        .add_node(Node::new(NodeType::Receipt, serde_json::json!({})))
        .unwrap();
    g.add_edge(Edge::new(EdgeType::CausalLink { strength: 0.9 }, a, b))
        .unwrap();
    g.add_proof(Proof::new("test claim"));

    let bytes = serialize_graph(&g).unwrap();
    let restored = deserialize_graph(&bytes).unwrap();
    assert_eq!(restored.node_count(), 2);
    assert_eq!(restored.edge_count(), 1);
    assert_eq!(restored.proof_count(), 1);
}

#[test]
fn domain_vocab_integrates_with_graph() {
    let mut reg = VocabRegistry::new();
    let vocab = DomainVocab::new("finance").with_node_type(
        "Trade",
        "A financial trade",
        serde_json::json!({}),
    );
    reg.register(vocab).unwrap();

    assert!(reg.is_known_node_type("finance", "Trade"));

    let mut g = PrimeGraph::new();
    g.add_node(Node::new(
        NodeType::Domain {
            domain: "finance".into(),
            name: "Trade".into(),
        },
        serde_json::json!({"symbol": "AAPL", "qty": 100}),
    ))
    .unwrap();

    assert_eq!(g.node_count(), 1);
}
