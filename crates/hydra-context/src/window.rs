//! Context window — ordered collection of context items with TTL expiry.

use crate::constants::{CONTEXT_WINDOW_MAX_ITEMS, CONTEXT_WINDOW_TTL_SECONDS};
use crate::errors::ContextError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single item within a context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// The content of this context item.
    pub content: String,
    /// How significant this item is (0.0 to 1.0).
    pub significance: f64,
    /// The domain this item belongs to, if any.
    pub domain: Option<String>,
    /// When this item was created.
    pub timestamp: DateTime<Utc>,
}

impl ContextItem {
    /// Create a new context item.
    pub fn new(content: impl Into<String>, significance: f64) -> Self {
        Self {
            content: content.into(),
            significance: significance.clamp(0.0, 1.0),
            domain: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a context item with a domain tag.
    pub fn with_domain(
        content: impl Into<String>,
        significance: f64,
        domain: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            significance: significance.clamp(0.0, 1.0),
            domain: Some(domain.into()),
            timestamp: Utc::now(),
        }
    }
}

/// A time-bounded, significance-ordered collection of context items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Label for this window (e.g., "active", "historical").
    pub label: String,
    /// Items ordered by significance descending.
    pub items: Vec<ContextItem>,
    /// When this window was created.
    pub created_at: DateTime<Utc>,
}

impl ContextWindow {
    /// Create a new empty context window with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Add an item, maintaining significance ordering and max-items cap.
    pub fn add(&mut self, item: ContextItem) {
        self.items.push(item);
        self.items.sort_by(|a, b| {
            b.significance
                .partial_cmp(&a.significance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.items.truncate(CONTEXT_WINDOW_MAX_ITEMS);
    }

    /// Check whether this window has expired.
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds();
        elapsed > CONTEXT_WINDOW_TTL_SECONDS as i64
    }

    /// Return items if the window is fresh, or an error if expired.
    pub fn fresh_items(&self) -> Result<&[ContextItem], ContextError> {
        if self.is_expired() {
            Err(ContextError::WindowExpired)
        } else {
            Ok(&self.items)
        }
    }

    /// Return the number of items in this window.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check whether this window is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_ordered_by_significance() {
        let mut w = ContextWindow::new("test");
        w.add(ContextItem::new("low", 0.2));
        w.add(ContextItem::new("high", 0.9));
        w.add(ContextItem::new("mid", 0.5));
        assert!(w.items[0].significance >= w.items[1].significance);
        assert!(w.items[1].significance >= w.items[2].significance);
    }

    #[test]
    fn respects_max_items() {
        let mut w = ContextWindow::new("test");
        for i in 0..60 {
            w.add(ContextItem::new(format!("item-{i}"), i as f64 / 100.0));
        }
        assert_eq!(w.len(), CONTEXT_WINDOW_MAX_ITEMS);
    }

    #[test]
    fn fresh_window_returns_items() {
        let w = ContextWindow::new("test");
        assert!(w.fresh_items().is_ok());
    }
}
