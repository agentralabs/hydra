//! Orientation — adding context alongside output without changing content.

use crate::graph::MeaningGraph;
use crate::temporal::TemporalHorizon;

/// Context that the soul can provide for orientation.
#[derive(Debug, Clone)]
pub struct OrientationContext {
    /// Top meaning labels from the orientation vector.
    pub top_meanings: Vec<String>,
    /// Current confidence level (0.0-1.0).
    pub confidence: f64,
    /// Dominant temporal horizon.
    pub horizon: TemporalHorizon,
    /// Whether the soul is ready to speak.
    pub ready: bool,
}

impl OrientationContext {
    /// A silent context — the soul has nothing to say yet.
    pub fn silent() -> Self {
        Self {
            top_meanings: Vec::new(),
            confidence: 0.0,
            horizon: TemporalHorizon::Immediate,
            ready: false,
        }
    }

    /// Build an orientation context from a meaning graph and horizon.
    pub fn new(graph: &MeaningGraph, horizon: TemporalHorizon) -> Self {
        let vector = graph.orientation_vector();
        let top_meanings: Vec<String> = vector.iter().map(|n| n.label.clone()).collect();
        let confidence = graph.orientation_confidence();
        let ready = graph.is_ready_to_speak();
        Self {
            top_meanings,
            confidence,
            horizon,
            ready,
        }
    }
}

/// Output with orientation applied alongside.
///
/// CRITICAL INVARIANT: `apply()` NEVER changes the content.
/// It only adds orientation context alongside.
#[derive(Debug, Clone)]
pub struct OrientedOutput {
    /// The original content, unchanged.
    pub content: String,
    /// Orientation context provided alongside.
    pub context: OrientationContext,
}

impl OrientedOutput {
    /// Apply orientation to content. Content is NEVER modified.
    ///
    /// If the soul is not ready, the context is silent.
    pub fn apply(content: impl Into<String>, context: OrientationContext) -> Self {
        Self {
            content: content.into(),
            context,
        }
    }
}

/// Generate a human-readable orientation summary.
pub fn orientation_summary(ctx: &OrientationContext) -> String {
    if !ctx.ready {
        return String::from("(soul: accumulating — not yet ready to orient)");
    }

    let meanings = if ctx.top_meanings.is_empty() {
        String::from("none yet")
    } else {
        ctx.top_meanings.join(", ")
    };

    format!(
        "(soul: oriented — confidence {:.0}%, horizon {:?}, top meanings: {})",
        ctx.confidence * 100.0,
        ctx.horizon,
        meanings
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_never_changes() {
        let original = "Hello, world!";
        let output = OrientedOutput::apply(original, OrientationContext::silent());
        assert_eq!(output.content, original, "content must never change");
    }

    #[test]
    fn silent_summary() {
        let ctx = OrientationContext::silent();
        let summary = orientation_summary(&ctx);
        assert!(
            summary.contains("accumulating"),
            "silent soul should say accumulating"
        );
    }
}
