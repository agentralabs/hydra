use std::collections::HashMap;
use std::sync::Arc;

use crate::bridge::*;
use crate::registry::SisterRegistry;

/// Pending call for batching
#[derive(Debug)]
struct PendingCall {
    action: SisterAction,
}

/// Sister call batcher — groups calls by sister, executes in parallel
pub struct SisterBatcher {
    pending: HashMap<SisterId, Vec<PendingCall>>,
    registry: Arc<SisterRegistry>,
    individual_calls: u64,
    batched_calls: u64,
}

impl SisterBatcher {
    pub fn new(registry: Arc<SisterRegistry>) -> Self {
        Self {
            pending: HashMap::new(),
            registry,
            individual_calls: 0,
            batched_calls: 0,
        }
    }

    /// Queue a call for batching
    pub fn queue(&mut self, sister_id: SisterId, action: SisterAction) {
        self.pending
            .entry(sister_id)
            .or_default()
            .push(PendingCall { action });
        self.individual_calls += 1;
    }

    /// Flush all pending calls, grouped by sister, executed in parallel
    pub async fn flush_all(&mut self) -> HashMap<SisterId, Vec<Result<SisterResult, SisterError>>> {
        let mut results = HashMap::new();
        let pending = std::mem::take(&mut self.pending);

        for (sister_id, calls) in pending {
            let actions: Vec<SisterAction> = calls.into_iter().map(|c| c.action).collect();

            if let Some(bridge) = self.registry.get(sister_id) {
                self.batched_calls += 1;
                let batch_results = bridge.batch_call(actions).await;
                results.insert(sister_id, batch_results);
            } else {
                // Sister not available — return errors for all actions
                let errors: Vec<Result<SisterResult, SisterError>> = actions
                    .iter()
                    .map(|_| {
                        Err(SisterError {
                            sister_id,
                            message: format!(
                                "{} is not registered. Check sister configuration.",
                                sister_id.name()
                            ),
                            retryable: false,
                        })
                    })
                    .collect();
                results.insert(sister_id, errors);
            }
        }

        results
    }

    /// Flush calls for a specific sister
    pub async fn flush(&mut self, sister_id: SisterId) -> Vec<Result<SisterResult, SisterError>> {
        let calls = self.pending.remove(&sister_id).unwrap_or_default();
        let actions: Vec<SisterAction> = calls.into_iter().map(|c| c.action).collect();

        if actions.is_empty() {
            return vec![];
        }

        if let Some(bridge) = self.registry.get(sister_id) {
            self.batched_calls += 1;
            bridge.batch_call(actions).await
        } else {
            actions
                .iter()
                .map(|_| {
                    Err(SisterError {
                        sister_id,
                        message: format!("{} not registered.", sister_id.name()),
                        retryable: false,
                    })
                })
                .collect()
        }
    }

    /// Number of pending calls
    pub fn pending_count(&self) -> usize {
        self.pending.values().map(|v| v.len()).sum()
    }

    /// Token savings estimate: batched vs individual
    pub fn tokens_saved(&self) -> u64 {
        // Each batch call saves overhead vs individual calls
        // Estimate: 50 tokens overhead per individual call
        let overhead_per_call: u64 = 50;
        if self.batched_calls > 0 {
            (self.individual_calls.saturating_sub(self.batched_calls)) * overhead_per_call
        } else {
            0
        }
    }

    /// Stats for metrics
    pub fn individual_calls(&self) -> u64 {
        self.individual_calls
    }

    pub fn batched_calls(&self) -> u64 {
        self.batched_calls
    }
}
