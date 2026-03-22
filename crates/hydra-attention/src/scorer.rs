//! Item scoring for attention allocation.
//!
//! Scores context items from all five windows, applying bonuses
//! for urgency, resonance, domain match, and window source.

use crate::constants::{RESONANCE_SIGNIFICANCE_BONUS, URGENCY_SIGNIFICANCE_BONUS};
use hydra_context::{ContextItem, ContextWindow};
use serde::{Deserialize, Serialize};

/// A context item scored for attention allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredItem {
    /// The original content.
    pub content: String,
    /// The base significance score from the context item.
    pub base_score: f64,
    /// The final score after bonuses.
    pub final_score: f64,
    /// Bonuses applied, as (label, value) pairs.
    pub bonuses: Vec<(String, f64)>,
    /// The domain of the original item, if any.
    pub domain: Option<String>,
}

impl ScoredItem {
    /// Score a single context item with optional bonuses.
    ///
    /// Applies urgency and resonance bonuses based on item properties.
    pub fn from_context_item(item: &ContextItem, urgency: f64, has_resonance: bool) -> Self {
        let base_score = item.significance;
        let mut bonuses = Vec::new();
        let mut bonus_total = 0.0;

        if urgency >= 0.7 {
            bonuses.push(("urgency".to_string(), URGENCY_SIGNIFICANCE_BONUS));
            bonus_total += URGENCY_SIGNIFICANCE_BONUS;
        }

        if has_resonance {
            bonuses.push(("resonance".to_string(), RESONANCE_SIGNIFICANCE_BONUS));
            bonus_total += RESONANCE_SIGNIFICANCE_BONUS;
        }

        let final_score = (base_score + bonus_total).min(1.0);

        Self {
            content: item.content.clone(),
            base_score,
            final_score,
            bonuses,
            domain: item.domain.clone(),
        }
    }
}

/// Score all items from all five context windows.
///
/// Each window type applies different bonuses:
/// - Active: +0.1 boost (current focus)
/// - Anomalies: +0.15 boost (unexpected patterns need attention)
/// - Gaps: +0.1 for domain match
/// - Predicted: base score only
/// - Historical: base score only
///
/// Returns items sorted by final_score descending.
#[allow(clippy::too_many_arguments)]
pub fn score_all_items(
    active: &ContextWindow,
    historical: &ContextWindow,
    predicted: &ContextWindow,
    gaps: &ContextWindow,
    anomalies: &ContextWindow,
    urgency: f64,
    has_resonance: bool,
    input_domain: Option<&str>,
) -> Vec<ScoredItem> {
    let mut scored = Vec::new();

    // Score active items with +0.1 boost.
    for item in &active.items {
        let mut si = ScoredItem::from_context_item(item, urgency, has_resonance);
        si.bonuses.push(("active-window".to_string(), 0.1));
        si.final_score = (si.final_score + 0.1).min(1.0);
        scored.push(si);
    }

    // Score anomaly items with +0.15 boost.
    for item in &anomalies.items {
        let mut si = ScoredItem::from_context_item(item, urgency, has_resonance);
        si.bonuses.push(("anomaly-window".to_string(), 0.15));
        si.final_score = (si.final_score + 0.15).min(1.0);
        scored.push(si);
    }

    // Score gap items with +0.1 for domain match.
    for item in &gaps.items {
        let mut si = ScoredItem::from_context_item(item, urgency, has_resonance);
        if let (Some(item_domain), Some(in_domain)) = (&item.domain, input_domain) {
            if item_domain == in_domain {
                si.bonuses.push(("domain-match".to_string(), 0.1));
                si.final_score = (si.final_score + 0.1).min(1.0);
            }
        }
        scored.push(si);
    }

    // Score predicted items at base.
    for item in &predicted.items {
        let si = ScoredItem::from_context_item(item, urgency, has_resonance);
        scored.push(si);
    }

    // Score historical items at base.
    for item in &historical.items {
        let si = ScoredItem::from_context_item(item, urgency, has_resonance);
        scored.push(si);
    }

    // Sort by final_score descending.
    scored.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_window(label: &str, items: Vec<(&str, f64)>) -> ContextWindow {
        let mut w = ContextWindow::new(label);
        for (content, sig) in items {
            w.add(ContextItem::new(content, sig));
        }
        w
    }

    #[test]
    fn active_items_boosted() {
        let active = make_window("active", vec![("test", 0.5)]);
        let empty = ContextWindow::new("empty");
        let scored = score_all_items(&active, &empty, &empty, &empty, &empty, 0.3, false, None);
        assert_eq!(scored.len(), 1);
        assert!(scored[0].final_score > scored[0].base_score);
    }

    #[test]
    fn anomaly_items_boosted() {
        let empty = ContextWindow::new("empty");
        let anomalies = make_window("anomalies", vec![("odd pattern", 0.6)]);
        let scored = score_all_items(&empty, &empty, &empty, &empty, &anomalies, 0.3, false, None);
        assert_eq!(scored.len(), 1);
        assert!(scored[0].final_score > scored[0].base_score);
    }

    #[test]
    fn sorted_by_final_score() {
        let active = make_window("active", vec![("low", 0.1)]);
        let anomalies = make_window("anomalies", vec![("high", 0.9)]);
        let empty = ContextWindow::new("empty");
        let scored = score_all_items(
            &active, &empty, &empty, &empty, &anomalies, 0.3, false, None,
        );
        assert!(scored[0].final_score >= scored[1].final_score);
    }
}
