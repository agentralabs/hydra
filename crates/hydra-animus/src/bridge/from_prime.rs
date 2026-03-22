//! Convert an Animus Prime graph into human-readable text.

use crate::{
    errors::AnimusError,
    graph::{NodeType, PrimeGraph},
};

/// A human-readable rendering of a Prime graph.
#[derive(Debug, Clone)]
pub struct HumanReadable {
    /// Short summary suitable for TUI display.
    pub summary: String,
    /// Detailed description if more context is needed.
    pub detail: Option<String>,
    /// Confidence level of the content (0.0-1.0).
    pub confidence: f64,
}

/// Convert a Prime graph to a human-readable summary.
pub fn graph_to_text(graph: &PrimeGraph) -> Result<HumanReadable, AnimusError> {
    if graph.is_empty() {
        return Ok(HumanReadable {
            summary: "(empty graph)".to_string(),
            detail: None,
            confidence: 1.0,
        });
    }

    let intent_nodes = graph.nodes_of_type(&NodeType::Intent);
    let belief_nodes = graph.nodes_of_type(&NodeType::Belief);

    let summary = if !intent_nodes.is_empty() {
        intent_nodes
            .iter()
            .filter_map(|n| n.content.get("raw_text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("; ")
    } else if !belief_nodes.is_empty() {
        format!("{} belief(s)", belief_nodes.len())
    } else {
        format!(
            "{} node(s), {} edge(s), {} proof(s)",
            graph.node_count(),
            graph.edge_count(),
            graph.proof_count()
        )
    };

    let detail = if graph.node_count() > 1 {
        Some(format!(
            "Graph contains {} nodes, {} edges, {} proofs",
            graph.node_count(),
            graph.edge_count(),
            graph.proof_count()
        ))
    } else {
        None
    };

    let confidence = 1.0; // simplified for v0.1.0

    Ok(HumanReadable {
        summary,
        detail,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Node, NodeType};

    #[test]
    fn empty_graph_summarized() {
        let g = PrimeGraph::new();
        let h = graph_to_text(&g).unwrap();
        assert!(h.summary.contains("empty"));
    }

    #[test]
    fn intent_node_text_extracted() {
        let mut g = PrimeGraph::new();
        g.add_node(Node::new(
            NodeType::Intent,
            serde_json::json!({"raw_text": "deploy now"}),
        ))
        .unwrap();
        let h = graph_to_text(&g).unwrap();
        assert!(h.summary.contains("deploy now"));
    }

    #[test]
    fn multi_node_graph_has_detail() {
        let mut g = PrimeGraph::new();
        g.add_node(Node::new(NodeType::Belief, serde_json::json!({})))
            .unwrap();
        g.add_node(Node::new(NodeType::Receipt, serde_json::json!({})))
            .unwrap();
        let h = graph_to_text(&g).unwrap();
        assert!(h.detail.is_some());
    }
}
