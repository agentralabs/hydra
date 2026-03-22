//! Attention allocator — assigns processing depth to scored items.
//!
//! Takes scored items and a budget, assigns Full/Summary/Filtered
//! based on significance thresholds and budget availability.

use crate::budget::{AttentionBudget, ProcessingDepth};
use crate::constants::{
    MAX_FOCUS_ITEMS, MAX_SUMMARY_ITEMS, MIN_SIGNIFICANCE_FOR_ATTENTION,
    MIN_SIGNIFICANCE_FOR_FULL_DEPTH,
};
use crate::scorer::ScoredItem;

/// An item with its assigned processing depth.
#[derive(Debug, Clone)]
pub struct AllocatedItem {
    /// The scored item.
    pub item: ScoredItem,
    /// The assigned processing depth.
    pub depth: ProcessingDepth,
}

/// Allocate processing depth to scored items within a budget.
///
/// Items are processed in score order (highest first). Each item
/// is assigned Full, Summary, or Filtered based on:
/// 1. Whether it meets the minimum significance threshold
/// 2. Whether it meets the full-depth threshold
/// 3. Whether the budget can afford the processing depth
/// 4. Whether the focus/summary caps have been reached
pub fn allocate(scored: &[ScoredItem], budget: &mut AttentionBudget) -> Vec<AllocatedItem> {
    let mut result = Vec::new();
    let mut focus_count: usize = 0;
    let mut summary_count: usize = 0;

    for item in scored {
        // Below minimum significance — filter out.
        if item.final_score < MIN_SIGNIFICANCE_FOR_ATTENTION {
            result.push(AllocatedItem {
                item: item.clone(),
                depth: ProcessingDepth::Filtered,
            });
            continue;
        }

        // Try full depth first.
        if item.final_score >= MIN_SIGNIFICANCE_FOR_FULL_DEPTH
            && focus_count < MAX_FOCUS_ITEMS
            && budget.can_afford(ProcessingDepth::Full)
            && budget.consume(ProcessingDepth::Full).is_ok()
        {
            focus_count += 1;
            result.push(AllocatedItem {
                item: item.clone(),
                depth: ProcessingDepth::Full,
            });
            continue;
        }

        // Try summary depth.
        if summary_count < MAX_SUMMARY_ITEMS
            && budget.can_afford(ProcessingDepth::Summary)
            && budget.consume(ProcessingDepth::Summary).is_ok()
        {
            summary_count += 1;
            result.push(AllocatedItem {
                item: item.clone(),
                depth: ProcessingDepth::Summary,
            });
            continue;
        }

        // Budget exhausted or caps reached — filter.
        result.push(AllocatedItem {
            item: item.clone(),
            depth: ProcessingDepth::Filtered,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scorer::ScoredItem;
    use hydra_language::{AffectSignal, IntentKind, InteractionRegister};

    fn neutral_affect() -> AffectSignal {
        AffectSignal {
            register: InteractionRegister::Neutral,
            confidence: 0.7,
            keywords_detected: vec![],
        }
    }

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
    fn high_score_gets_full() {
        let scored = vec![make_scored(0.8)];
        let mut budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral_affect());
        let allocated = allocate(&scored, &mut budget);
        assert_eq!(allocated.len(), 1);
        assert_eq!(allocated[0].depth, ProcessingDepth::Full);
    }

    #[test]
    fn low_score_gets_filtered() {
        let scored = vec![make_scored(0.05)];
        let mut budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral_affect());
        let allocated = allocate(&scored, &mut budget);
        assert_eq!(allocated[0].depth, ProcessingDepth::Filtered);
    }

    #[test]
    fn mid_score_gets_summary() {
        let scored = vec![make_scored(0.3)];
        let mut budget = AttentionBudget::compute(&IntentKind::AnalysisRequest, &neutral_affect());
        let allocated = allocate(&scored, &mut budget);
        assert_eq!(allocated[0].depth, ProcessingDepth::Summary);
    }
}
