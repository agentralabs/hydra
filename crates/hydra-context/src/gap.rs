//! Gap context — tracks what Hydra knows it does not know.

use crate::constants::{GAP_SIGNIFICANCE_THRESHOLD, MAX_ACTIVE_GAPS};
use crate::window::{ContextItem, ContextWindow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A signal indicating a gap in Hydra's knowledge or capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSignal {
    /// Description of the gap.
    pub description: String,
    /// How significant this gap is (0.0 to 1.0).
    pub significance: f64,
    /// Domain where the gap was detected, if any.
    pub domain: Option<String>,
    /// When this gap was detected.
    pub detected_at: DateTime<Utc>,
}

impl GapSignal {
    /// Create a new gap signal.
    pub fn new(description: impl Into<String>, significance: f64) -> Self {
        Self {
            description: description.into(),
            significance: significance.clamp(0.0, 1.0),
            domain: None,
            detected_at: Utc::now(),
        }
    }

    /// Create a gap signal with a domain tag.
    pub fn with_domain(
        description: impl Into<String>,
        significance: f64,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            significance: significance.clamp(0.0, 1.0),
            domain: Some(domain.into()),
            detected_at: Utc::now(),
        }
    }
}

/// Tracks active knowledge gaps.
#[derive(Debug, Clone)]
pub struct GapContext {
    /// Active gap signals.
    gaps: Vec<GapSignal>,
}

impl GapContext {
    /// Create a new empty gap context.
    pub fn new() -> Self {
        Self { gaps: Vec::new() }
    }

    /// Add a gap signal if it meets the significance threshold.
    ///
    /// Gaps below the threshold are silently ignored.
    /// If at capacity, the least significant gap is replaced.
    pub fn add_gap(&mut self, gap: GapSignal) {
        if gap.significance < GAP_SIGNIFICANCE_THRESHOLD {
            return;
        }

        if self.gaps.len() < MAX_ACTIVE_GAPS {
            self.gaps.push(gap);
        } else {
            // Replace least significant if new one is more significant
            if let Some(min_idx) = self
                .gaps
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.significance
                        .partial_cmp(&b.significance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
            {
                if self.gaps[min_idx].significance < gap.significance {
                    self.gaps[min_idx] = gap;
                }
            }
        }
    }

    /// Return the number of active gaps.
    pub fn len(&self) -> usize {
        self.gaps.len()
    }

    /// Check whether there are no active gaps.
    pub fn is_empty(&self) -> bool {
        self.gaps.is_empty()
    }

    /// Build a context window from active gaps.
    pub fn build_window(&self) -> ContextWindow {
        let mut window = ContextWindow::new("gaps");
        for gap in &self.gaps {
            let item = match &gap.domain {
                Some(d) => ContextItem::with_domain(&gap.description, gap.significance, d),
                None => ContextItem::new(&gap.description, gap.significance),
            };
            window.add(item);
        }
        window
    }
}

impl Default for GapContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::GAP_SIGNIFICANCE_THRESHOLD;

    #[test]
    fn rejects_below_threshold() {
        let mut ctx = GapContext::new();
        ctx.add_gap(GapSignal::new(
            "insignificant",
            GAP_SIGNIFICANCE_THRESHOLD - 0.1,
        ));
        assert!(ctx.is_empty());
    }

    #[test]
    fn accepts_above_threshold() {
        let mut ctx = GapContext::new();
        ctx.add_gap(GapSignal::new(
            "significant",
            GAP_SIGNIFICANCE_THRESHOLD + 0.1,
        ));
        assert_eq!(ctx.len(), 1);
    }

    #[test]
    fn respects_max_active() {
        let mut ctx = GapContext::new();
        for i in 0..15 {
            ctx.add_gap(GapSignal::new(
                format!("gap-{i}"),
                GAP_SIGNIFICANCE_THRESHOLD + (i as f64) * 0.02,
            ));
        }
        assert!(ctx.len() <= MAX_ACTIVE_GAPS);
    }

    #[test]
    fn build_window_creates_items() {
        let mut ctx = GapContext::new();
        ctx.add_gap(GapSignal::new("missing data", 0.7));
        let window = ctx.build_window();
        assert_eq!(window.len(), 1);
    }
}
