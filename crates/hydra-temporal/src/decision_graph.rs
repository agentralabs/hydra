//! Decision graph — a DAG of temporal constraints.

use crate::constants::DECISION_GRAPH_MAX_DEPTH;
use crate::constraint::{DecisionConstraint, DecisionId};
use crate::errors::TemporalError;
use std::collections::{HashMap, HashSet};

/// A conflict detected when checking a proposed action.
#[derive(Debug, Clone)]
pub struct ConstraintConflict {
    /// The decision that produced the conflict.
    pub decision_id: DecisionId,
    /// Human-readable reason for the conflict.
    pub reason: String,
    /// Current strength of the conflicting constraint.
    pub strength: f64,
}

/// A directed acyclic graph of decision constraints.
///
/// Nodes are `DecisionConstraint` values; edges are parent-child links.
/// This structure is append-only — decisions cannot be removed.
pub struct DecisionGraph {
    nodes: HashMap<String, DecisionConstraint>,
    children: HashMap<String, Vec<String>>,
}

impl Default for DecisionGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionGraph {
    /// Create a new, empty decision graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// Record a new decision in the graph.
    ///
    /// Returns an error if the parent does not exist or would create a cycle.
    pub fn record(&mut self, constraint: DecisionConstraint) -> Result<(), TemporalError> {
        let id = constraint.id.as_str().to_string();

        // Validate parent exists if specified
        if let Some(ref parent) = constraint.parent {
            let parent_key = parent.as_str().to_string();
            if !self.nodes.contains_key(&parent_key) {
                return Err(TemporalError::DecisionNotFound(parent_key));
            }
            self.children
                .entry(parent_key)
                .or_default()
                .push(id.clone());
        }

        self.nodes.insert(id, constraint);
        Ok(())
    }

    /// Retrieve a decision by its ID.
    pub fn get(&self, id: &DecisionId) -> Option<&DecisionConstraint> {
        self.nodes.get(id.as_str())
    }

    /// Check all active constraints against a proposed action.
    pub fn check_conflicts(
        &self,
        proposed_action: &str,
        elapsed_seconds: f64,
    ) -> Vec<ConstraintConflict> {
        let mut conflicts = Vec::new();
        for constraint in self.nodes.values() {
            if let Some(reason) = constraint.check_conflict(proposed_action, elapsed_seconds) {
                conflicts.push(ConstraintConflict {
                    decision_id: constraint.id.clone(),
                    reason,
                    strength: constraint.current_strength(elapsed_seconds),
                });
            }
        }
        conflicts
    }

    /// Return all constraints that are still active, sorted by strength
    /// (strongest first).
    pub fn active_constraints(&self, elapsed_seconds: f64) -> Vec<&DecisionConstraint> {
        let mut active: Vec<_> = self
            .nodes
            .values()
            .filter(|c| {
                c.decay
                    .as_ref()
                    .map(|d| d.is_active(elapsed_seconds))
                    .unwrap_or(true)
            })
            .collect();
        active.sort_by(|a, b| {
            b.current_strength(elapsed_seconds)
                .partial_cmp(&a.current_strength(elapsed_seconds))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        active
    }

    /// Return all constraints that have decayed to fossil level.
    pub fn fossil_constraints(&self, elapsed_seconds: f64) -> Vec<&DecisionConstraint> {
        self.nodes
            .values()
            .filter(|c| {
                c.decay
                    .as_ref()
                    .map(|d| d.is_fossil(elapsed_seconds))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Total number of decisions recorded.
    pub fn total_decisions(&self) -> usize {
        self.nodes.len()
    }

    /// DFS traversal of the subtree rooted at the given decision ID.
    ///
    /// Returns all decision IDs reachable from the root.
    pub fn subtree(&self, root: &DecisionId) -> Result<Vec<DecisionId>, TemporalError> {
        let root_key = root.as_str().to_string();
        if !self.nodes.contains_key(&root_key) {
            return Err(TemporalError::DecisionNotFound(root_key));
        }

        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![(root_key.clone(), 0usize)];

        while let Some((current, depth)) = stack.pop() {
            if depth > DECISION_GRAPH_MAX_DEPTH {
                return Err(TemporalError::GraphDepthExceeded(DECISION_GRAPH_MAX_DEPTH));
            }
            if !visited.insert(current.clone()) {
                continue;
            }
            result.push(DecisionId::from_value(&current));
            if let Some(kids) = self.children.get(&current) {
                for kid in kids {
                    stack.push((kid.clone(), depth + 1));
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::ConstraintKind;
    use crate::timestamp::Timestamp;

    fn make_constraint(
        id: &str,
        kind: ConstraintKind,
        pattern: &str,
        parent: Option<&str>,
    ) -> DecisionConstraint {
        DecisionConstraint::new(
            DecisionId::from_value(id),
            Timestamp::now(),
            kind,
            format!("Test constraint: {id}"),
            pattern.to_string(),
            parent.map(DecisionId::from_value),
            1.0,
        )
    }

    #[test]
    fn record_and_retrieve() {
        let mut graph = DecisionGraph::new();
        let c = make_constraint("d1", ConstraintKind::Informational, "test", None);
        graph.record(c).unwrap();
        assert!(graph.get(&DecisionId::from_value("d1")).is_some());
    }

    #[test]
    fn conflict_detection() {
        let mut graph = DecisionGraph::new();
        let c = make_constraint("d1", ConstraintKind::Forbids, "rm -rf", None);
        graph.record(c).unwrap();
        let conflicts = graph.check_conflicts("run rm -rf /", 0.0);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn subtree_traversal() {
        let mut graph = DecisionGraph::new();
        graph
            .record(make_constraint(
                "root",
                ConstraintKind::Informational,
                "x",
                None,
            ))
            .unwrap();
        graph
            .record(make_constraint(
                "child1",
                ConstraintKind::Informational,
                "x",
                Some("root"),
            ))
            .unwrap();
        graph
            .record(make_constraint(
                "child2",
                ConstraintKind::Informational,
                "x",
                Some("root"),
            ))
            .unwrap();
        let subtree = graph.subtree(&DecisionId::from_value("root")).unwrap();
        assert_eq!(subtree.len(), 3);
    }
}
