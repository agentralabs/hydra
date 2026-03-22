//! Attention frame — the output of the attention allocation pipeline.
//!
//! Separates allocated items into focus, summary, and filtered sets,
//! with budget tracking and summary reporting.

use crate::allocator::AllocatedItem;
use crate::budget::{AttentionBudget, ProcessingDepth};
use crate::scorer::ScoredItem;
use serde::{Deserialize, Serialize};

/// The output of a single attention allocation cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionFrame {
    /// Items receiving full-depth processing.
    pub focus_items: Vec<ScoredItem>,
    /// Items receiving summary-depth processing.
    pub summary_items: Vec<ScoredItem>,
    /// Number of items that were filtered out.
    pub filtered_count: usize,
    /// The budget state after allocation.
    pub budget: AttentionBudget,
}

impl AttentionFrame {
    /// Build a frame from allocated items and the budget.
    pub fn from_allocated(allocated: Vec<AllocatedItem>, budget: AttentionBudget) -> Self {
        let mut focus_items = Vec::new();
        let mut summary_items = Vec::new();
        let mut filtered_count = 0;

        for alloc in allocated {
            match alloc.depth {
                ProcessingDepth::Full => focus_items.push(alloc.item),
                ProcessingDepth::Summary => summary_items.push(alloc.item),
                ProcessingDepth::Filtered => filtered_count += 1,
            }
        }

        Self {
            focus_items,
            summary_items,
            filtered_count,
            budget,
        }
    }

    /// Return the total number of attended items (focus + summary).
    pub fn attended_count(&self) -> usize {
        self.focus_items.len() + self.summary_items.len()
    }

    /// Check whether there are any focus items.
    pub fn has_focus(&self) -> bool {
        !self.focus_items.is_empty()
    }

    /// Return the top focus item, if any.
    pub fn top_focus(&self) -> Option<&ScoredItem> {
        self.focus_items.first()
    }

    /// Return the budget utilization ratio.
    pub fn utilization(&self) -> f64 {
        self.budget.utilization()
    }

    /// Return a TUI-friendly summary of this frame.
    pub fn summary(&self) -> String {
        format!(
            "attention: focus={} summary={} filtered={} utilization={:.1}%",
            self.focus_items.len(),
            self.summary_items.len(),
            self.filtered_count,
            self.utilization() * 100.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scored(score: f64) -> ScoredItem {
        ScoredItem {
            content: format!("item-{score}"),
            base_score: score,
            final_score: score,
            bonuses: vec![],
            domain: None,
        }
    }

    #[test]
    fn from_allocated_separates_correctly() {
        let allocated = vec![
            AllocatedItem {
                item: make_scored(0.9),
                depth: ProcessingDepth::Full,
            },
            AllocatedItem {
                item: make_scored(0.4),
                depth: ProcessingDepth::Summary,
            },
            AllocatedItem {
                item: make_scored(0.05),
                depth: ProcessingDepth::Filtered,
            },
        ];
        let budget = AttentionBudget::compute(
            &hydra_language::IntentKind::AnalysisRequest,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.7,
                keywords_detected: vec![],
            },
        );
        let frame = AttentionFrame::from_allocated(allocated, budget);
        assert_eq!(frame.focus_items.len(), 1);
        assert_eq!(frame.summary_items.len(), 1);
        assert_eq!(frame.filtered_count, 1);
        assert_eq!(frame.attended_count(), 2);
    }

    #[test]
    fn summary_format() {
        let budget = AttentionBudget::compute(
            &hydra_language::IntentKind::StatusQuery,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.7,
                keywords_detected: vec![],
            },
        );
        let frame = AttentionFrame::from_allocated(vec![], budget);
        let s = frame.summary();
        assert!(s.contains("attention:"));
        assert!(s.contains("focus="));
        assert!(s.contains("utilization="));
    }

    #[test]
    fn utilization_bounds() {
        let budget = AttentionBudget::compute(
            &hydra_language::IntentKind::AnalysisRequest,
            &hydra_language::AffectSignal {
                register: hydra_language::InteractionRegister::Neutral,
                confidence: 0.7,
                keywords_detected: vec![],
            },
        );
        let frame = AttentionFrame::from_allocated(vec![], budget);
        assert!(frame.utilization() >= 0.0);
        assert!(frame.utilization() <= 1.0);
    }
}
