//! Convert human-readable text intent into an Animus Prime graph.

use crate::{
    errors::AnimusError,
    graph::{Edge, EdgeType, Node, NodeType, PrimeGraph},
    semiring::signal::{Signal, SignalId, SignalTier, SignalWeight},
};

/// A resolved intent from human text.
#[derive(Debug, Clone)]
pub struct ResolvedIntent {
    /// The original text.
    pub raw: String,
    /// A Prime graph representing the intent.
    pub graph: PrimeGraph,
    /// The signal wrapping this intent.
    pub signal: Signal,
}

/// Convert a raw text command from the principal into a Prime graph and Signal.
pub fn text_to_signal(
    raw_text: &str,
    source_tier: u8,
    caused_by: SignalId,
) -> Result<ResolvedIntent, AnimusError> {
    let mut graph = PrimeGraph::new();

    // Create the intent node
    let intent_node = Node::new(
        NodeType::Intent,
        serde_json::json!({
            "raw_text": raw_text,
            "source": "principal_text_input"
        }),
    );
    let intent_id = graph.add_node(intent_node)?;

    // Create a receipt node (every intent produces a receipt trace)
    let receipt_node = Node::new(
        NodeType::Receipt,
        serde_json::json!({
            "action": "intent.received",
            "text":   raw_text
        }),
    );
    let receipt_id = graph.add_node(receipt_node)?;

    // Link intent to receipt causally
    graph.add_edge(Edge::new(
        EdgeType::CausalLink { strength: 1.0 },
        intent_id,
        receipt_id,
    ))?;

    let signal = Signal::new(
        graph.clone(),
        caused_by,
        SignalWeight::max(),
        SignalTier::Fleet,
        source_tier,
    );

    Ok(ResolvedIntent {
        raw: raw_text.to_string(),
        graph,
        signal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_converts_to_signal() {
        let result = text_to_signal("build agentic-data v0.2.0", 2, SignalId::identity());
        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.raw, "build agentic-data v0.2.0");
        assert!(!intent.signal.is_orphan());
        assert_eq!(intent.graph.node_count(), 2);
        assert_eq!(intent.graph.edge_count(), 1);
    }

    #[test]
    fn signal_has_correct_tier() {
        let result = text_to_signal("test", 2, SignalId::identity()).unwrap();
        assert_eq!(result.signal.tier, SignalTier::Fleet);
    }

    #[test]
    fn chain_is_complete() {
        let result = text_to_signal("hello", 2, SignalId::identity()).unwrap();
        assert!(result.signal.chain_is_complete());
    }
}
