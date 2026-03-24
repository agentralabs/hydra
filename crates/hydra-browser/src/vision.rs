//! VisionProvider trait — decouples browser/desktop from LLM implementation.
//!
//! The kernel provides a concrete implementation wrapping its LlmCaller.
//! Tests provide a mock. No circular dependency.

use async_trait::async_trait;
use crate::errors::BrowserError;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Vision analysis provider — sends images to an LLM for understanding.
#[async_trait]
pub trait VisionProvider: Send + Sync {
    /// Analyze an image and return the model's text response.
    async fn analyze_image(
        &self,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String, BrowserError>;
}

/// Tracks vision API call budget to prevent runaway costs.
#[derive(Debug, Clone)]
pub struct VisionBudget {
    calls_this_hour: Arc<AtomicU32>,
    max_per_hour: u32,
    hour_start: Arc<std::sync::Mutex<chrono::DateTime<chrono::Utc>>>,
}

impl VisionBudget {
    pub fn new(max_per_hour: u32) -> Self {
        Self {
            calls_this_hour: Arc::new(AtomicU32::new(0)),
            max_per_hour,
            hour_start: Arc::new(std::sync::Mutex::new(chrono::Utc::now())),
        }
    }

    /// Check if a vision call is allowed. Returns true if within budget.
    pub fn try_consume(&self) -> bool {
        if self.max_per_hour == 0 {
            return true; // unlimited
        }

        let now = chrono::Utc::now();
        let mut start = self.hour_start.lock().unwrap();

        // Reset counter if hour has passed
        if (now - *start).num_seconds() >= 3600 {
            *start = now;
            self.calls_this_hour.store(0, Ordering::SeqCst);
        }

        let current = self.calls_this_hour.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_per_hour {
            self.calls_this_hour.fetch_sub(1, Ordering::SeqCst);
            return false;
        }
        true
    }

    pub fn remaining(&self) -> u32 {
        let used = self.calls_this_hour.load(Ordering::SeqCst);
        self.max_per_hour.saturating_sub(used)
    }
}

/// A budgeted vision provider that wraps another provider with rate limiting.
pub struct BudgetedVision<V: VisionProvider> {
    inner: V,
    budget: VisionBudget,
}

impl<V: VisionProvider> BudgetedVision<V> {
    pub fn new(inner: V, budget: VisionBudget) -> Self {
        Self { inner, budget }
    }
}

#[async_trait]
impl<V: VisionProvider> VisionProvider for BudgetedVision<V> {
    async fn analyze_image(
        &self,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String, BrowserError> {
        if !self.budget.try_consume() {
            return Err(BrowserError::VisionError(format!(
                "Vision budget exhausted ({} calls/hour). {} remaining",
                self.budget.max_per_hour,
                self.budget.remaining()
            )));
        }
        self.inner.analyze_image(image_bytes, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_allows_within_limit() {
        let budget = VisionBudget::new(5);
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(!budget.try_consume()); // 6th call rejected
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn unlimited_budget_always_allows() {
        let budget = VisionBudget::new(0);
        for _ in 0..1000 {
            assert!(budget.try_consume());
        }
    }
}
