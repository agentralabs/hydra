use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Identifier for a sister in the batch queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchSisterId {
    Memory,
    Vision,
    Codebase,
    Identity,
    Time,
    Contract,
    Comm,
    Planning,
    Cognition,
    Reality,
    Forge,
    Aegis,
    Veritas,
    Evolve,
}

impl BatchSisterId {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Vision => "vision",
            Self::Codebase => "codebase",
            Self::Identity => "identity",
            Self::Time => "time",
            Self::Contract => "contract",
            Self::Comm => "comm",
            Self::Planning => "planning",
            Self::Cognition => "cognition",
            Self::Reality => "reality",
            Self::Forge => "forge",
            Self::Aegis => "aegis",
            Self::Veritas => "veritas",
            Self::Evolve => "evolve",
        }
    }
}

/// A call queued for batching
#[derive(Debug, Clone)]
pub struct BatchCall {
    pub tool: String,
    pub params: serde_json::Value,
    pub queued_at: Instant,
}

/// Configuration for the batch queue
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of calls to batch together per sister
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing a batch
    pub flush_timeout: Duration,
    /// Estimated token overhead per individual call (saved by batching)
    pub overhead_per_call: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10,
            flush_timeout: Duration::from_millis(50),
            overhead_per_call: 50,
        }
    }
}

/// Result of flushing a batch
#[derive(Debug, Clone)]
pub struct BatchFlushResult {
    pub sister_id: BatchSisterId,
    pub calls: Vec<BatchCall>,
    pub batch_count: usize,
    pub individual_count: usize,
}

impl BatchFlushResult {
    /// Estimated tokens saved by batching these calls
    pub fn tokens_saved(&self, overhead_per_call: u64) -> u64 {
        if self.individual_count <= 1 {
            return 0;
        }
        (self.individual_count as u64 - 1) * overhead_per_call
    }
}

/// Sister call batch queue — groups calls by sister for efficient execution
pub struct BatchQueue {
    queues: HashMap<BatchSisterId, Vec<BatchCall>>,
    config: BatchConfig,
    total_queued: u64,
    total_flushed: u64,
    total_batches: u64,
}

impl BatchQueue {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            queues: HashMap::new(),
            config,
            total_queued: 0,
            total_flushed: 0,
            total_batches: 0,
        }
    }

    /// Queue a call for a specific sister
    pub fn enqueue(&mut self, sister_id: BatchSisterId, tool: impl Into<String>, params: serde_json::Value) {
        let call = BatchCall {
            tool: tool.into(),
            params,
            queued_at: Instant::now(),
        };
        self.queues.entry(sister_id).or_default().push(call);
        self.total_queued += 1;
    }

    /// Check if any sister's queue should be flushed (hit max size or timeout)
    pub fn sisters_ready_to_flush(&self) -> Vec<BatchSisterId> {
        let mut ready = Vec::new();
        for (sister_id, calls) in &self.queues {
            if calls.is_empty() {
                continue;
            }
            // Flush if batch is full
            if calls.len() >= self.config.max_batch_size {
                ready.push(*sister_id);
                continue;
            }
            // Flush if oldest call has waited too long
            if let Some(oldest) = calls.first() {
                if oldest.queued_at.elapsed() >= self.config.flush_timeout {
                    ready.push(*sister_id);
                }
            }
        }
        ready
    }

    /// Flush the queue for a specific sister, returning the batch
    pub fn flush(&mut self, sister_id: BatchSisterId) -> Option<BatchFlushResult> {
        let calls = self.queues.remove(&sister_id)?;
        if calls.is_empty() {
            return None;
        }
        let individual_count = calls.len();
        self.total_flushed += individual_count as u64;
        self.total_batches += 1;
        Some(BatchFlushResult {
            sister_id,
            calls,
            batch_count: 1,
            individual_count,
        })
    }

    /// Flush all queues, returning results grouped by sister
    pub fn flush_all(&mut self) -> Vec<BatchFlushResult> {
        let sisters: Vec<BatchSisterId> = self.queues.keys().cloned().collect();
        let mut results = Vec::new();
        for sister_id in sisters {
            if let Some(result) = self.flush(sister_id) {
                results.push(result);
            }
        }
        results
    }

    /// Total pending calls across all sisters
    pub fn pending_count(&self) -> usize {
        self.queues.values().map(|v| v.len()).sum()
    }

    /// Pending calls for a specific sister
    pub fn pending_for(&self, sister_id: BatchSisterId) -> usize {
        self.queues.get(&sister_id).map_or(0, |v| v.len())
    }

    /// Whether any calls are pending
    pub fn has_pending(&self) -> bool {
        self.queues.values().any(|v| !v.is_empty())
    }

    /// Total calls ever queued
    pub fn total_queued(&self) -> u64 {
        self.total_queued
    }

    /// Total calls ever flushed
    pub fn total_flushed(&self) -> u64 {
        self.total_flushed
    }

    /// Total batches executed
    pub fn total_batches(&self) -> u64 {
        self.total_batches
    }

    /// Estimated total tokens saved by batching
    pub fn total_tokens_saved(&self) -> u64 {
        if self.total_batches == 0 {
            return 0;
        }
        // Each batch saves (individual_count - 1) * overhead
        // Approximate: total_flushed - total_batches = calls that avoided overhead
        self.total_flushed
            .saturating_sub(self.total_batches)
            * self.config.overhead_per_call
    }

    /// Number of distinct sisters with pending calls
    pub fn active_sisters(&self) -> usize {
        self.queues.values().filter(|v| !v.is_empty()).count()
    }
}

#[cfg(test)]
#[path = "batching_tests.rs"]
mod tests;
