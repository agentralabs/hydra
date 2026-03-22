//! The Final Layer 1 Test Harness for hydra-soul.
//!
//! Tests the complete soul orientation layer.

use hydra_soul::constants::ORIENTATION_CONFIDENCE_THRESHOLD;
use hydra_soul::deepening::DeepeningState;
use hydra_soul::graph::MeaningGraph;
use hydra_soul::node::{MeaningNode, NodeKind};
use hydra_soul::orient::{orientation_summary, OrientationContext, OrientedOutput};
use hydra_soul::soul::Soul;
use hydra_soul::temporal::{TemporalHorizon, TemporalSignals};

fn main() {
    println!("=== Phase 15: The Final Layer 1 — hydra-soul ===\n");

    test_empty_graph_silence();
    test_first_exchange();
    test_repeated_reinforcement();
    test_confidence_growth();
    test_node_weight_ordering();
    test_deepening_lifecycle();
    test_reflection_enforcement();
    test_deepening_rejection();
    test_temporal_classification();
    test_silent_passthrough();
    test_soul_accumulation();
    test_status_line();
    test_content_never_changes();
    test_node_kind_multipliers();
    test_decay_respects_floor();
    test_orientation_vector_limit();
    test_graph_default();
    test_soul_default();
    test_temporal_care_multipliers();
    test_deepening_store_active();
    test_soul_temporal_horizon();
    test_orientation_summary_silent();
    test_orientation_summary_ready();
    test_fossil_detection();
    test_never_abandoned_heaviest();

    println!("\n=== ALL PHASE 15 TESTS PASSED ===");
    println!("=== LAYER 1 COMPLETE — THE SOUL IS ALIVE ===");
}

fn test_empty_graph_silence() {
    print!("  empty graph is silent ... ");
    let g = MeaningGraph::new();
    assert!(!g.is_ready_to_speak());
    assert_eq!(g.orientation_confidence(), 0.0);
    assert_eq!(g.node_count(), 0);
    println!("OK");
}

fn test_first_exchange() {
    print!("  first exchange creates a node ... ");
    let mut g = MeaningGraph::new();
    g.record_exchange("reliability", NodeKind::RecurringChoice)
        .expect("record");
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.exchange_count(), 1);
    let node = g.all_nodes().next().expect("one node");
    assert_eq!(node.label, "reliability");
    assert_eq!(node.reinforcement_count, 1);
    println!("OK");
}

fn test_repeated_reinforcement() {
    print!("  repeated label reinforces ... ");
    let mut g = MeaningGraph::new();
    g.record_exchange("care", NodeKind::RecurringReturn)
        .expect("first");
    let w1 = g.all_nodes().next().expect("node").weight;
    g.record_exchange("care", NodeKind::RecurringReturn)
        .expect("second");
    let w2 = g.all_nodes().next().expect("node").weight;
    assert!(w2 > w1, "weight must increase on reinforcement");
    assert_eq!(g.node_count(), 1, "no duplicate nodes");
    assert_eq!(g.exchange_count(), 2);
    println!("OK");
}

fn test_confidence_growth() {
    print!("  confidence grows with exchanges ... ");
    let mut g = MeaningGraph::new();
    let c0 = g.orientation_confidence();
    for i in 0..100 {
        g.record_exchange(&format!("topic_{i}"), NodeKind::RecurringChoice)
            .expect("record");
    }
    let c100 = g.orientation_confidence();
    assert!(c100 > c0, "confidence must grow");
    assert!(c100 < 1.0, "100 exchanges should not saturate");
    println!("OK (confidence at {c100:.4})");
}

fn test_node_weight_ordering() {
    print!("  orientation vector ordered by weight ... ");
    let mut g = MeaningGraph::new();
    g.record_exchange("low", NodeKind::RecurringChoice)
        .expect("add");
    // Reinforce "high" many times
    for _ in 0..10 {
        g.record_exchange("high", NodeKind::RecurringProtection)
            .expect("add");
    }
    let vector = g.orientation_vector();
    assert!(!vector.is_empty());
    assert_eq!(vector[0].label, "high", "highest weight first");
    println!("OK");
}

fn test_deepening_lifecycle() {
    print!("  deepening full lifecycle (0-day reflection) ... ");
    let mut soul = Soul::new();
    let id = soul
        .propose_deepening_with_reflection("Always explain why", 0)
        .expect("propose");

    let rec = soul.deepening_mut().get_mut(&id).expect("find");
    assert_eq!(rec.state, DeepeningState::Proposed);

    rec.reconfirm().expect("reconfirm");
    assert_eq!(rec.state, DeepeningState::AwaitingCoherenceAssessment);

    rec.assess_coherence(0.95).expect("assess");
    assert_eq!(rec.state, DeepeningState::AwaitingFinalConfirmation);

    rec.finalize(true).expect("finalize");
    assert_eq!(rec.state, DeepeningState::Deepened);

    assert_eq!(soul.deepening().total_deepened(), 1);
    println!("OK");
}

fn test_reflection_enforcement() {
    print!("  reflection period enforced ... ");
    let mut soul = Soul::new();
    let id = soul
        .propose_deepening_with_reflection("Needs time", 365)
        .expect("propose");

    let rec = soul.deepening_mut().get_mut(&id).expect("find");
    let result = rec.reconfirm();
    assert!(result.is_err(), "should fail — reflection not elapsed");
    println!("OK");
}

fn test_deepening_rejection() {
    print!("  deepening rejection ... ");
    let mut soul = Soul::new();
    let id = soul
        .propose_deepening_with_reflection("Bad idea", 0)
        .expect("propose");

    let rec = soul.deepening_mut().get_mut(&id).expect("find");
    rec.reconfirm().expect("reconfirm");
    rec.assess_coherence(0.2).expect("assess");
    rec.finalize(false).expect("reject");
    assert_eq!(rec.state, DeepeningState::Rejected);
    assert_eq!(soul.deepening().total_deepened(), 0);
    println!("OK");
}

fn test_temporal_classification() {
    print!("  temporal classification ... ");
    let signals = TemporalSignals {
        immediate_count: 1,
        developmental_count: 10,
        foundational_count: 3,
        generational_count: 0,
    };
    assert_eq!(signals.classify(), TemporalHorizon::Developmental);

    let all_equal = TemporalSignals {
        immediate_count: 5,
        developmental_count: 5,
        foundational_count: 5,
        generational_count: 5,
    };
    assert_eq!(
        all_equal.classify(),
        TemporalHorizon::Generational,
        "ties break toward longer horizon"
    );
    println!("OK");
}

fn test_silent_passthrough() {
    print!("  silent passthrough preserves content ... ");
    let soul = Soul::new();
    let output = soul.orient("Hello, world!");
    assert_eq!(output.content, "Hello, world!");
    assert!(!output.context.ready);
    println!("OK");
}

fn test_soul_accumulation() {
    print!("  soul accumulates silently ... ");
    let mut soul = Soul::new();
    for i in 0..50 {
        soul.record_exchange(&format!("topic_{i}"), NodeKind::RecurringChoice)
            .expect("record");
    }
    let ctx = soul.orientation_context();
    // 50 exchanges is not enough to reach the threshold
    assert!(!ctx.ready, "50 exchanges should not make soul ready");
    println!("OK");
}

fn test_status_line() {
    print!("  status line content ... ");
    let soul = Soul::new();
    let status = soul.status_line();
    assert!(
        status.contains("accumulating"),
        "empty soul should show accumulating"
    );

    println!("OK (status: {status})");
}

fn test_content_never_changes() {
    print!("  OrientedOutput never changes content ... ");
    let content = "The quick brown fox jumps over the lazy dog.";
    let ctx = OrientationContext::silent();
    let output = OrientedOutput::apply(content, ctx);
    assert_eq!(output.content, content);
    println!("OK");
}

fn test_node_kind_multipliers() {
    print!("  node kind multipliers ordered correctly ... ");
    assert!(
        NodeKind::RecurringProtection.base_multiplier()
            > NodeKind::RecurringChoice.base_multiplier()
    );
    assert!(
        NodeKind::NeverAbandoned.base_multiplier()
            > NodeKind::RecurringProtection.base_multiplier()
    );
    println!("OK");
}

fn test_decay_respects_floor() {
    print!("  decay never below floor ... ");
    let mut node = MeaningNode::new("test", NodeKind::RecurringChoice);
    node.decay(1_000_000.0);
    assert!(
        node.weight >= hydra_soul::constants::NODE_WEIGHT_FLOOR,
        "weight must not drop below floor"
    );
    assert!(node.is_fossil(), "heavily decayed node is a fossil");
    println!("OK");
}

fn test_orientation_vector_limit() {
    print!("  orientation vector limited to K ... ");
    let mut g = MeaningGraph::new();
    for i in 0..20 {
        g.record_exchange(&format!("node_{i}"), NodeKind::RecurringChoice)
            .expect("record");
    }
    let vector = g.orientation_vector();
    assert!(
        vector.len() <= hydra_soul::constants::ORIENTATION_VECTOR_K,
        "vector must not exceed K"
    );
    println!("OK (vector len = {})", vector.len());
}

fn test_graph_default() {
    print!("  graph default is empty ... ");
    let g = MeaningGraph::default();
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.exchange_count(), 0);
    println!("OK");
}

fn test_soul_default() {
    print!("  soul default is empty ... ");
    let soul = Soul::default();
    assert_eq!(soul.graph().node_count(), 0);
    assert_eq!(soul.graph().exchange_count(), 0);
    assert_eq!(soul.deepening().total_deepened(), 0);
    println!("OK");
}

fn test_temporal_care_multipliers() {
    print!("  temporal care multipliers increase ... ");
    assert!(TemporalHorizon::Generational.care_multiplier() > 1.0);
    assert!(
        TemporalHorizon::Foundational.care_multiplier()
            > TemporalHorizon::Developmental.care_multiplier()
    );
    println!("OK");
}

fn test_deepening_store_active() {
    print!("  deepening store tracks active proposals ... ");
    let mut soul = Soul::new();
    let _ = soul
        .propose_deepening_with_reflection("Pending one", 365)
        .expect("propose");
    let id2 = soul
        .propose_deepening_with_reflection("Done one", 0)
        .expect("propose");

    // Complete the second one
    let rec = soul.deepening_mut().get_mut(&id2).expect("find");
    rec.reconfirm().expect("reconfirm");
    rec.assess_coherence(0.9).expect("assess");
    rec.finalize(true).expect("finalize");

    assert_eq!(soul.deepening().active().len(), 1);
    assert_eq!(soul.deepening().total_deepened(), 1);
    println!("OK");
}

fn test_soul_temporal_horizon() {
    print!("  soul temporal horizon updates ... ");
    let mut soul = Soul::new();
    assert_eq!(soul.temporal_horizon(), TemporalHorizon::Immediate);

    soul.set_temporal_signals(TemporalSignals {
        immediate_count: 0,
        developmental_count: 0,
        foundational_count: 10,
        generational_count: 0,
    });
    assert_eq!(soul.temporal_horizon(), TemporalHorizon::Foundational);
    println!("OK");
}

fn test_orientation_summary_silent() {
    print!("  orientation summary when silent ... ");
    let ctx = OrientationContext::silent();
    let summary = orientation_summary(&ctx);
    assert!(summary.contains("accumulating"));
    println!("OK");
}

fn test_orientation_summary_ready() {
    print!("  orientation summary when ready ... ");
    let ctx = OrientationContext {
        top_meanings: vec!["reliability".into(), "care".into()],
        confidence: ORIENTATION_CONFIDENCE_THRESHOLD + 0.1,
        horizon: TemporalHorizon::Foundational,
        ready: true,
    };
    let summary = orientation_summary(&ctx);
    assert!(summary.contains("oriented"));
    assert!(summary.contains("reliability"));
    println!("OK ({summary})");
}

fn test_fossil_detection() {
    print!("  fossil detection works ... ");
    let mut node = MeaningNode::new("ancient", NodeKind::RecurringChoice);
    assert!(!node.is_fossil(), "fresh node is not a fossil");
    node.decay(1_000_000.0);
    assert!(node.is_fossil(), "fully decayed node is a fossil");
    println!("OK");
}

fn test_never_abandoned_heaviest() {
    print!("  NeverAbandoned is heaviest kind ... ");
    let choice = MeaningNode::new("c", NodeKind::RecurringChoice);
    let ret = MeaningNode::new("r", NodeKind::RecurringReturn);
    let prot = MeaningNode::new("p", NodeKind::RecurringProtection);
    let commit = MeaningNode::new("m", NodeKind::RecurringCommitment);
    let never = MeaningNode::new("n", NodeKind::NeverAbandoned);
    assert!(never.weight > choice.weight);
    assert!(never.weight > ret.weight);
    assert!(never.weight > prot.weight);
    assert!(never.weight > commit.weight);
    println!("OK");
}
