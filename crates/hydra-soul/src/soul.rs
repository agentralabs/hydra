//! The Soul coordinator — single entry point for all soul operations.

use crate::deepening::DeepeningStore;
use crate::errors::SoulError;
use crate::graph::MeaningGraph;
use crate::node::NodeKind;
use crate::orient::{OrientationContext, OrientedOutput};
use crate::temporal::{TemporalHorizon, TemporalSignals};

/// The Soul — Hydra's orientation layer.
///
/// All external writes go through `record_exchange()`.
/// Everything else is read-only.
#[derive(Debug, Clone)]
pub struct Soul {
    graph: MeaningGraph,
    deepening: DeepeningStore,
    temporal: TemporalSignals,
}

impl Soul {
    /// Create a new soul with empty state.
    pub fn new() -> Self {
        Self {
            graph: MeaningGraph::new(),
            deepening: DeepeningStore::new(),
            temporal: TemporalSignals::default(),
        }
    }

    /// Record an exchange — the single write entry point.
    ///
    /// All external crates must use this method to add meaning.
    pub fn record_exchange(&mut self, label: &str, kind: NodeKind) -> Result<(), SoulError> {
        self.graph.record_exchange(label, kind)
    }

    /// Get the current orientation context (read-only).
    pub fn orientation_context(&self) -> OrientationContext {
        let horizon = self.temporal.classify();
        if !self.graph.is_ready_to_speak() {
            return OrientationContext::silent();
        }
        OrientationContext::new(&self.graph, horizon)
    }

    /// Orient output — adds context alongside without changing content.
    pub fn orient(&self, content: impl Into<String>) -> OrientedOutput {
        let ctx = self.orientation_context();
        OrientedOutput::apply(content, ctx)
    }

    /// Propose a constitutional deepening.
    pub fn propose_deepening(&mut self, principle: impl Into<String>) -> Result<String, SoulError> {
        self.deepening.propose(principle)
    }

    /// Propose a deepening with custom reflection period (for tests).
    pub fn propose_deepening_with_reflection(
        &mut self,
        principle: impl Into<String>,
        min_reflection_days: i64,
    ) -> Result<String, SoulError> {
        self.deepening
            .propose_with_reflection(principle, min_reflection_days)
    }

    /// Get the deepening store (read-only).
    pub fn deepening(&self) -> &DeepeningStore {
        &self.deepening
    }

    /// Get the deepening store (mutable, for lifecycle operations).
    pub fn deepening_mut(&mut self) -> &mut DeepeningStore {
        &mut self.deepening
    }

    /// Get the meaning graph (read-only).
    pub fn graph(&self) -> &MeaningGraph {
        &self.graph
    }

    /// Update temporal signals.
    pub fn set_temporal_signals(&mut self, signals: TemporalSignals) {
        self.temporal = signals;
    }

    /// Get the current temporal horizon classification.
    pub fn temporal_horizon(&self) -> TemporalHorizon {
        self.temporal.classify()
    }

    /// Status line for display.
    ///
    /// Shows "accumulating" when the soul is silent,
    /// "oriented" when it is ready to speak.
    pub fn status_line(&self) -> String {
        let exchanges = self.graph.exchange_count();
        let nodes = self.graph.node_count();
        let confidence = self.graph.orientation_confidence();

        if self.graph.is_ready_to_speak() {
            format!(
                "soul: oriented ({} exchanges, {} nodes, {:.0}% confidence)",
                exchanges,
                nodes,
                confidence * 100.0
            )
        } else {
            format!(
                "soul: accumulating ({} exchanges, {} nodes, {:.0}% confidence)",
                exchanges,
                nodes,
                confidence * 100.0
            )
        }
    }
}

impl Default for Soul {
    fn default() -> Self {
        Self::new()
    }
}
