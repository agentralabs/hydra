//! Causal World Model — build and traverse causal belief graphs with
//! Bayesian confidence propagation. Store causal edges in Memory sister.
//!
//! Why isn't a sister doing this? Memory sister stores edges but doesn't
//! do Bayesian propagation. This module owns the causal reasoning logic.

use crate::sisters::SistersHandle;
use std::collections::HashMap;

/// A causal edge: cause → effect with strength.
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub cause: String,
    pub effect: String,
    pub strength: f64,
    pub edge_type: CausalType,
}

/// Type of causal relationship.
#[derive(Debug, Clone, PartialEq)]
pub enum CausalType {
    Causes,
    Inhibits,
    Correlates,
}

/// A node in the propagated causal tree.
#[derive(Debug, Clone)]
pub struct CausalNode {
    pub concept: String,
    pub confidence: f64,
    pub depth: u32,
    pub path: Vec<String>,
    pub children: Vec<CausalNode>,
}

/// Result of a causal propagation.
#[derive(Debug, Clone)]
pub struct CausalTree {
    pub trigger: String,
    pub root: CausalNode,
    pub total_nodes: usize,
    pub max_depth: u32,
    pub gaps: Vec<MissingLink>,
}

/// A gap in the causal model.
#[derive(Debug, Clone)]
pub struct MissingLink {
    pub from: String,
    pub expected_effect: String,
    pub reason: String,
}

/// In-memory causal graph.
#[derive(Debug, Default)]
pub struct CausalGraph {
    edges: Vec<CausalEdge>,
    index: HashMap<String, Vec<usize>>,
}

impl CausalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a causal edge to the graph.
    pub fn add_edge(&mut self, cause: &str, effect: &str, strength: f64, edge_type: CausalType) {
        let idx = self.edges.len();
        self.edges.push(CausalEdge {
            cause: cause.to_string(),
            effect: effect.to_string(),
            strength,
            edge_type,
        });
        self.index.entry(cause.to_string()).or_default().push(idx);
    }

    /// Propagate causation from a trigger through the graph via BFS.
    /// Multiplies confidence at each step. Stops when confidence < threshold.
    pub fn propagate(&self, trigger: &str, max_depth: u32, threshold: f64) -> CausalTree {
        let root = self.propagate_node(trigger, 1.0, 0, max_depth, threshold, &mut Vec::new());
        let total = count_nodes(&root);
        let max_d = max_node_depth(&root);

        CausalTree {
            trigger: trigger.to_string(),
            root,
            total_nodes: total,
            max_depth: max_d,
            gaps: Vec::new(),
        }
    }

    fn propagate_node(
        &self,
        concept: &str,
        confidence: f64,
        depth: u32,
        max_depth: u32,
        threshold: f64,
        visited: &mut Vec<String>,
    ) -> CausalNode {
        let mut node = CausalNode {
            concept: concept.to_string(),
            confidence,
            depth,
            path: visited.clone(),
            children: Vec::new(),
        };

        if depth >= max_depth || confidence < threshold {
            return node;
        }

        visited.push(concept.to_string());

        if let Some(edge_indices) = self.index.get(concept) {
            for &idx in edge_indices {
                let edge = &self.edges[idx];
                if visited.contains(&edge.effect) {
                    continue; // Avoid cycles
                }

                let child_confidence = match edge.edge_type {
                    CausalType::Causes => confidence * edge.strength,
                    CausalType::Inhibits => confidence * (1.0 - edge.strength),
                    CausalType::Correlates => confidence * edge.strength * 0.7,
                };

                if child_confidence >= threshold {
                    let child = self.propagate_node(
                        &edge.effect, child_confidence, depth + 1,
                        max_depth, threshold, visited,
                    );
                    node.children.push(child);
                }
            }
        }

        visited.pop();
        node
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Store a causal edge in Memory sister for persistence.
pub async fn store_causal_edge(
    sisters: &SistersHandle,
    cause: &str,
    effect: &str,
    strength: f64,
    edge_type: &str,
) {
    let content = format!(
        "[causal] {} {} {} (strength: {:.2})",
        cause, edge_type, effect, strength,
    );
    sisters.memory_workspace_add(&content, "causal-model").await;
    eprintln!("[hydra:causal] Stored: {} → {} ({:.2})", cause, effect, strength);
}

/// Format a causal tree for injection into the LLM prompt.
pub fn format_causal_tree(tree: &CausalTree) -> String {
    let mut output = format!("Causal analysis from '{}':\n", tree.trigger);
    format_node(&tree.root, &mut output, 0);
    if !tree.gaps.is_empty() {
        output.push_str("\nGaps in causal model:\n");
        for gap in &tree.gaps {
            output.push_str(&format!("  {} → ? (expected: {})\n", gap.from, gap.expected_effect));
        }
    }
    output
}

fn format_node(node: &CausalNode, output: &mut String, indent: usize) {
    let prefix = "  ".repeat(indent);
    if indent > 0 {
        output.push_str(&format!("{}→ {} ({:.0}%)\n", prefix, node.concept, node.confidence * 100.0));
    }
    for child in &node.children {
        format_node(child, output, indent + 1);
    }
}

fn count_nodes(node: &CausalNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn max_node_depth(node: &CausalNode) -> u32 {
    if node.children.is_empty() {
        node.depth
    } else {
        node.children.iter().map(max_node_depth).max().unwrap_or(node.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_propagate() {
        let mut graph = CausalGraph::new();
        graph.add_edge("rate_hike", "bond_decline", 0.90, CausalType::Causes);
        graph.add_edge("bond_decline", "rebalancing", 0.80, CausalType::Causes);
        graph.add_edge("rebalancing", "rotation", 0.70, CausalType::Causes);

        let tree = graph.propagate("rate_hike", 5, 0.3);
        assert_eq!(tree.total_nodes, 4); // root + 3 effects
        assert!(tree.root.children[0].confidence > 0.85);
    }

    #[test]
    fn test_confidence_decay() {
        let mut graph = CausalGraph::new();
        graph.add_edge("a", "b", 0.5, CausalType::Causes);
        graph.add_edge("b", "c", 0.5, CausalType::Causes);

        let tree = graph.propagate("a", 5, 0.3);
        // a→b: 0.5, b→c: 0.25 (below 0.3 threshold)
        assert_eq!(tree.total_nodes, 2); // root + b only
    }

    #[test]
    fn test_cycle_prevention() {
        let mut graph = CausalGraph::new();
        graph.add_edge("a", "b", 0.9, CausalType::Causes);
        graph.add_edge("b", "a", 0.9, CausalType::Causes); // cycle

        let tree = graph.propagate("a", 5, 0.3);
        assert!(tree.total_nodes <= 3);
    }

    #[test]
    fn test_inhibition() {
        let mut graph = CausalGraph::new();
        graph.add_edge("rate_hike", "growth_stocks", 0.80, CausalType::Inhibits);

        let tree = graph.propagate("rate_hike", 3, 0.1);
        // Inhibits: confidence = 1.0 * (1 - 0.80) = 0.20
        assert!(tree.root.children[0].confidence < 0.25);
    }
}
