//! Test harness for hydra-animus.
//! Runs 27 scenarios exercising the full Animus Prime runtime.

use hydra_animus::{
    bridge::{from_prime::graph_to_text, to_prime::text_to_signal},
    bus::{
        router::{route, RoutingDecision},
        signature::BusSigningKey,
        validate_for_bus,
    },
    graph::{Edge, EdgeType, Node, NodeType, PrimeGraph},
    semiring::{
        compose::compose,
        merge::merge,
        orphan::validate_chain,
        signal::{Signal, SignalId, SignalTier, SignalWeight},
        weight::{compute_weight, verify_coefficient_sum, WeightInputs},
    },
    serial::binary::{deserialize_graph, deserialize_signal, serialize_graph, serialize_signal},
    vocab::{
        base::{is_base_edge_type, is_base_node_type},
        domain::{DomainVocab, VocabRegistry},
        growth::{
            growth_layer, growth_node_type, is_growth_edge_type, is_growth_node_type, GrowthLayer,
        },
    },
};

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! scenario {
        ($name:expr, $body:block) => {{
            let name = $name;
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
                Ok(_) => {
                    passed += 1;
                    eprintln!("  PASS: {}", name);
                }
                Err(_) => {
                    failed += 1;
                    eprintln!("  FAIL: {}", name);
                }
            }
        }};
    }

    eprintln!("=== Animus Prime Test Harness ===");
    eprintln!();

    // 1. Empty graph creation
    scenario!("1. Empty graph creation", {
        let g = PrimeGraph::new();
        assert!(g.is_empty());
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    });

    // 2. Node and edge addition
    scenario!("2. Node and edge addition", {
        let mut g = PrimeGraph::new();
        let a = g
            .add_node(Node::new(NodeType::Intent, serde_json::json!("hello")))
            .unwrap();
        let b = g
            .add_node(Node::new(NodeType::Receipt, serde_json::json!("world")))
            .unwrap();
        g.add_edge(Edge::new(EdgeType::CausalLink { strength: 1.0 }, a, b))
            .unwrap();
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    });

    // 3. Edge with unknown node rejected
    scenario!("3. Edge with unknown node rejected", {
        let mut g = PrimeGraph::new();
        let ghost = hydra_animus::NodeId::from_str("ghost");
        let real = g
            .add_node(Node::new(NodeType::Belief, serde_json::Value::Null))
            .unwrap();
        assert!(g
            .add_edge(Edge::new(EdgeType::References, ghost, real))
            .is_err());
    });

    // 4. Signal creation with identity parent
    scenario!("4. Signal creation with identity parent", {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        assert!(!s.is_orphan());
        assert!(s.chain_is_complete());
    });

    // 5. Constitutional identity signal
    scenario!("5. Constitutional identity signal", {
        let s = Signal::constitutional_identity();
        assert!(s.is_identity());
        assert!(s.chain_is_complete());
        assert!(validate_chain(&s).is_ok());
    });

    // 6. Orphan signal detection
    scenario!("6. Orphan signal detection", {
        let mut s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        s.causal_chain.clear();
        assert!(s.is_orphan());
        assert!(validate_chain(&s).is_err());
    });

    // 7. Causal composition
    scenario!("7. Causal composition", {
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
            SignalTier::Fleet,
            3,
        );
        let c = compose(&a, &b).unwrap();
        assert!(c.chain_is_complete());
    });

    // 8. Signal merge
    scenario!("8. Signal merge", {
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
        let c = merge(&a, &b).unwrap();
        assert!(c.chain_is_complete());
        assert_eq!(c.tier, SignalTier::Fleet);
    });

    // 9. Weight validation
    scenario!("9. Weight validation", {
        assert!(SignalWeight::new(0.0).is_err());
        assert!(SignalWeight::new(0.001).is_ok());
        assert!(SignalWeight::new(1.0).is_ok());
        assert!(SignalWeight::new(1.001).is_err());
    });

    // 10. Graph serialization round-trip
    scenario!("10. Graph serialization round-trip", {
        let mut g = PrimeGraph::new();
        g.add_node(Node::new(NodeType::Intent, serde_json::json!("test")))
            .unwrap();
        let bytes = serialize_graph(&g).unwrap();
        let restored = deserialize_graph(&bytes).unwrap();
        assert_eq!(restored.node_count(), 1);
    });

    // 11. Signal serialization round-trip
    scenario!("11. Signal serialization round-trip", {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        let bytes = serialize_signal(&s).unwrap();
        let restored = deserialize_signal(&bytes).unwrap();
        assert_eq!(restored.id, s.id);
    });

    // 12. Corrupted magic header rejected
    scenario!("12. Corrupted magic header rejected", {
        let g = PrimeGraph::new();
        let mut bytes = serialize_graph(&g).unwrap();
        bytes[0] = b'X';
        assert!(deserialize_graph(&bytes).is_err());
    });

    // 13. Ed25519 sign and verify
    scenario!("13. Ed25519 sign and verify", {
        let key = BusSigningKey::generate();
        let vk = key.verifying_key();
        let sig = key.sign("msg-001", b"payload");
        assert!(vk.verify("msg-001", b"payload", &sig).is_ok());
        assert!(vk.verify("msg-001", b"tampered", &sig).is_err());
    });

    // 14. Signal routing by tier
    scenario!("14. Signal routing by tier", {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Constitution,
            0,
        );
        assert_eq!(route(&s), RoutingDecision::ConstitutionImmediate);
    });

    // 15. Orphan signal dropped by router
    scenario!("15. Orphan signal dropped by router", {
        let mut s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        s.causal_chain.clear();
        assert!(matches!(route(&s), RoutingDecision::Drop { .. }));
    });

    // 16. Bus validation
    scenario!("16. Bus validation", {
        let s = Signal::new(
            PrimeGraph::new(),
            SignalId::identity(),
            SignalWeight::max(),
            SignalTier::Fleet,
            3,
        );
        assert!(validate_for_bus(&s).is_ok());
    });

    // 17. Base vocabulary check
    scenario!("17. Base vocabulary check", {
        assert!(is_base_node_type("Belief"));
        assert!(is_base_node_type("Intent"));
        assert!(!is_base_node_type("CustomThing"));
        assert!(is_base_edge_type("CausalLink"));
        assert!(!is_base_edge_type("CustomEdge"));
    });

    // 18. Domain vocabulary registration
    scenario!("18. Domain vocabulary registration", {
        let mut reg = VocabRegistry::new();
        let vocab =
            DomainVocab::new("finance").with_node_type("Trade", "A trade", serde_json::json!({}));
        reg.register(vocab).unwrap();
        assert!(reg.has_domain("finance"));
        assert!(reg.is_known_node_type("finance", "Trade"));
    });

    // 19. Text to signal bridge
    scenario!("19. Text to signal bridge", {
        let result = text_to_signal("deploy v2", 2, SignalId::identity()).unwrap();
        assert_eq!(result.raw, "deploy v2");
        assert_eq!(result.graph.node_count(), 2);
        assert!(result.signal.chain_is_complete());
    });

    // 20. Graph to text bridge
    scenario!("20. Graph to text bridge", {
        let mut g = PrimeGraph::new();
        g.add_node(Node::new(
            NodeType::Intent,
            serde_json::json!({"raw_text": "hello world"}),
        ))
        .unwrap();
        let h = graph_to_text(&g).unwrap();
        assert!(h.summary.contains("hello world"));
    });

    // 21. Growth node types recognized
    scenario!("21. Growth node types recognized", {
        assert!(is_growth_node_type("GenomeEntry"));
        assert!(is_growth_node_type("SystemProfile"));
        assert!(is_growth_node_type("AntifragileRecord"));
        assert!(is_growth_node_type("SynthesizedCapability"));
        assert!(is_growth_node_type("PlasticityProfile"));
        assert!(is_growth_node_type("CapabilityGrowthEvent"));
        assert!(is_growth_node_type("SituationSignature"));
        assert!(is_growth_node_type("ObstacleSignature"));
        assert!(is_growth_node_type("ApproachRecord"));
        assert!(is_growth_node_type("SystemClass"));
        assert!(is_growth_node_type("ProtocolFamily"));
        assert!(is_growth_node_type("InterfaceSignature"));
        assert!(is_growth_node_type("TopologyNeighbor"));
        assert!(!is_growth_node_type("Belief"));
    });

    // 22. Growth edge types recognized
    scenario!("22. Growth edge types recognized", {
        assert!(is_growth_edge_type("LearnedFrom"));
        assert!(is_growth_edge_type("SimilarTo"));
        assert!(is_growth_edge_type("ResolvedBy"));
        assert!(is_growth_edge_type("SynthesizedUsing"));
        assert!(is_growth_edge_type("AdaptedFor"));
        assert!(is_growth_edge_type("StrengthensFrom"));
        assert!(is_growth_edge_type("GrowsBy"));
        assert!(!is_growth_edge_type("CausalLink"));
    });

    // 23. Growth layer classification
    scenario!("23. Growth layer classification", {
        assert_eq!(growth_layer("GenomeEntry"), Some(GrowthLayer::Genome));
        assert_eq!(
            growth_layer("SystemProfile"),
            Some(GrowthLayer::Cartography)
        );
        assert_eq!(
            growth_layer("AntifragileRecord"),
            Some(GrowthLayer::Antifragile)
        );
        assert_eq!(
            growth_layer("SynthesizedCapability"),
            Some(GrowthLayer::Generative)
        );
        assert_eq!(
            growth_layer("PlasticityProfile"),
            Some(GrowthLayer::Plastic)
        );
        assert_eq!(growth_layer("Unknown"), None);
    });

    // 24. Growth types in base vocab (after integration)
    scenario!("24. Growth types in base vocab", {
        assert!(is_base_node_type("GenomeEntry"));
        assert!(is_base_edge_type("LearnedFrom"));
    });

    // 25. CCFT coefficients sum to 1.0
    scenario!("25. CCFT coefficients sum to 1.0", {
        assert!(verify_coefficient_sum());
    });

    // 26. CCFT weight computation
    scenario!("26. CCFT weight computation", {
        let high = WeightInputs::new(0, 1).with_novel().with_constitutional();
        let low = WeightInputs::new(5, 100);
        let w_high = compute_weight(&high).unwrap();
        let w_low = compute_weight(&low).unwrap();
        assert!(w_high.value() > w_low.value());
        assert!((0.001..=1.0).contains(&w_high.value()));
        assert!((0.001..=1.0).contains(&w_low.value()));
    });

    // 27. Growth nodes in graph
    scenario!("27. Growth nodes in graph", {
        let mut g = PrimeGraph::new();
        let genome = g
            .add_node(Node::new(
                growth_node_type("GenomeEntry"),
                serde_json::json!({"pattern": "ssh-retry"}),
            ))
            .unwrap();
        let skill = g
            .add_node(Node::new(
                NodeType::Skill,
                serde_json::json!({"name": "ssh-connect"}),
            ))
            .unwrap();
        g.add_edge(Edge::new(EdgeType::References, genome, skill))
            .unwrap();
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
    });

    eprintln!();
    eprintln!("=== Results: {} passed, {} failed ===", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
