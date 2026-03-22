//! Uncertainty Trees — structured confidence with weakest-link identification.
//!
//! Instead of one confidence number (0.72), produces a tree:
//!   Deploy safely (70%)
//!   ├── Tests pass (95%)
//!   │   ├── Unit tests (99%)
//!   │   └── Integration tests (85%)
//!   └── Config correct (70%) ← WEAKEST LINK
//!       └── Secrets rotated (70%)

use serde::{Deserialize, Serialize};

/// A single node in the uncertainty tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyNode {
    pub claim: String,
    pub confidence: f64,
    pub children: Vec<UncertaintyNode>,
}

impl UncertaintyNode {
    pub fn leaf(claim: impl Into<String>, confidence: f64) -> Self {
        Self {
            claim: claim.into(),
            confidence: confidence.clamp(0.0, 1.0),
            children: Vec::new(),
        }
    }

    pub fn with_children(
        claim: impl Into<String>,
        confidence: f64,
        children: Vec<UncertaintyNode>,
    ) -> Self {
        Self {
            claim: claim.into(),
            confidence: confidence.clamp(0.0, 1.0),
            children,
        }
    }

    /// Effective confidence: the weakest link in this subtree.
    /// A chain is as strong as its weakest part.
    pub fn effective_confidence(&self) -> f64 {
        if self.children.is_empty() {
            return self.confidence;
        }
        let child_min = self
            .children
            .iter()
            .map(|c| c.effective_confidence())
            .fold(f64::MAX, f64::min);
        self.confidence.min(child_min)
    }

    /// Find the weakest link in the entire subtree.
    pub fn weakest_link(&self) -> &UncertaintyNode {
        if self.children.is_empty() {
            return self;
        }
        let weakest_child = self
            .children
            .iter()
            .min_by(|a, b| {
                a.effective_confidence()
                    .partial_cmp(&b.effective_confidence())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap(); // safe: children is non-empty

        let child_weakest = weakest_child.weakest_link();
        if child_weakest.confidence < self.confidence {
            child_weakest
        } else {
            self
        }
    }
}

/// A complete uncertainty tree with report generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncertaintyTree {
    pub root: UncertaintyNode,
}

impl UncertaintyTree {
    pub fn new(root: UncertaintyNode) -> Self {
        Self { root }
    }

    /// Overall effective confidence (weakest path).
    pub fn effective_confidence(&self) -> f64 {
        self.root.effective_confidence()
    }

    /// The single claim that limits overall confidence.
    pub fn weakest_link(&self) -> &UncertaintyNode {
        self.root.weakest_link()
    }

    /// Build from a flat list of (claim, confidence) with optional parent index.
    pub fn from_claims(claims: Vec<(String, f64)>) -> Self {
        if claims.is_empty() {
            return Self::new(UncertaintyNode::leaf("no claims", 0.0));
        }
        if claims.len() == 1 {
            return Self::new(UncertaintyNode::leaf(&claims[0].0, claims[0].1));
        }
        let children: Vec<UncertaintyNode> = claims
            .iter()
            .skip(1)
            .map(|(claim, conf)| UncertaintyNode::leaf(claim, *conf))
            .collect();
        let root_conf = claims[0].1;
        Self::new(UncertaintyNode::with_children(
            &claims[0].0,
            root_conf,
            children,
        ))
    }

    /// Human-readable report for prompt injection.
    pub fn report(&self) -> String {
        let effective = self.effective_confidence();
        let weakest = self.weakest_link();
        let tree_str = format_tree(&self.root, 0);
        format!(
            "Confidence: {:.0}% (limited by: \"{}\" at {:.0}%)\n{}",
            effective * 100.0,
            weakest.claim,
            weakest.confidence * 100.0,
            tree_str
        )
    }
}

fn format_tree(node: &UncertaintyNode, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let prefix = if depth == 0 { "" } else { "├── " };
    let mut lines = vec![format!(
        "{}{}{} ({:.0}%)",
        indent,
        prefix,
        node.claim,
        node.confidence * 100.0
    )];
    for child in &node.children {
        lines.push(format_tree(child, depth + 1));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_effective_confidence() {
        let leaf = UncertaintyNode::leaf("test", 0.8);
        assert!((leaf.effective_confidence() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn weakest_link_is_child() {
        let tree = UncertaintyTree::new(UncertaintyNode::with_children(
            "root",
            0.95,
            vec![
                UncertaintyNode::leaf("strong", 0.90),
                UncertaintyNode::leaf("weak", 0.40),
            ],
        ));
        assert!((tree.effective_confidence() - 0.40).abs() < f64::EPSILON);
        assert_eq!(tree.weakest_link().claim, "weak");
    }

    #[test]
    fn from_claims_works() {
        let tree = UncertaintyTree::from_claims(vec![
            ("overall".into(), 0.9),
            ("step 1".into(), 0.95),
            ("step 2".into(), 0.60),
        ]);
        assert!((tree.effective_confidence() - 0.60).abs() < f64::EPSILON);
    }

    #[test]
    fn report_contains_weakest() {
        let tree = UncertaintyTree::from_claims(vec![
            ("deploy".into(), 0.95),
            ("tests".into(), 0.85),
            ("secrets".into(), 0.70),
        ]);
        let report = tree.report();
        assert!(report.contains("secrets"));
        assert!(report.contains("70%"));
    }
}
