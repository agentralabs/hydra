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
mod tests {
    use super::*;

    fn default_config() -> BatchConfig {
        BatchConfig {
            max_batch_size: 3,
            flush_timeout: Duration::from_millis(50),
            overhead_per_call: 50,
        }
    }

    // ── Queue basics ──────────────────────────────────────────

    #[test]
    fn new_queue_is_empty() {
        let queue = BatchQueue::new(default_config());
        assert_eq!(queue.pending_count(), 0);
        assert!(!queue.has_pending());
        assert_eq!(queue.total_queued(), 0);
        assert_eq!(queue.active_sisters(), 0);
    }

    #[test]
    fn enqueue_single_call() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        assert_eq!(queue.pending_count(), 1);
        assert!(queue.has_pending());
        assert_eq!(queue.total_queued(), 1);
        assert_eq!(queue.active_sisters(), 1);
    }

    #[test]
    fn enqueue_multiple_same_sister() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Memory, "memory_query", serde_json::json!({}));
        assert_eq!(queue.pending_count(), 2);
        assert_eq!(queue.pending_for(BatchSisterId::Memory), 2);
        assert_eq!(queue.active_sisters(), 1);
    }

    #[test]
    fn enqueue_multiple_different_sisters() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Vision, "vision_capture", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Codebase, "analyze", serde_json::json!({}));
        assert_eq!(queue.pending_count(), 3);
        assert_eq!(queue.active_sisters(), 3);
    }

    #[test]
    fn pending_for_returns_zero_for_unknown_sister() {
        let queue = BatchQueue::new(default_config());
        assert_eq!(queue.pending_for(BatchSisterId::Forge), 0);
    }

    // ── Flush single sister ───────────────────────────────────

    #[test]
    fn flush_returns_none_for_empty_sister() {
        let mut queue = BatchQueue::new(default_config());
        assert!(queue.flush(BatchSisterId::Memory).is_none());
    }

    #[test]
    fn flush_returns_queued_calls() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({"key": "val"}));
        queue.enqueue(BatchSisterId::Memory, "memory_query", serde_json::json!({}));
        let result = queue.flush(BatchSisterId::Memory).unwrap();
        assert_eq!(result.individual_count, 2);
        assert_eq!(result.batch_count, 1);
        assert_eq!(result.sister_id, BatchSisterId::Memory);
        assert_eq!(result.calls.len(), 2);
        assert_eq!(result.calls[0].tool, "memory_add");
        assert_eq!(result.calls[1].tool, "memory_query");
    }

    #[test]
    fn flush_clears_sister_queue() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        queue.flush(BatchSisterId::Memory);
        assert_eq!(queue.pending_for(BatchSisterId::Memory), 0);
        assert!(!queue.has_pending());
    }

    #[test]
    fn flush_does_not_affect_other_sisters() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Vision, "vision_capture", serde_json::json!({}));
        queue.flush(BatchSisterId::Memory);
        assert_eq!(queue.pending_for(BatchSisterId::Vision), 1);
    }

    // ── Flush all ─────────────────────────────────────────────

    #[test]
    fn flush_all_empty_returns_empty_vec() {
        let mut queue = BatchQueue::new(default_config());
        let results = queue.flush_all();
        assert!(results.is_empty());
    }

    #[test]
    fn flush_all_returns_all_sisters() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "memory_add", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Vision, "vision_capture", serde_json::json!({}));
        let results = queue.flush_all();
        assert_eq!(results.len(), 2);
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn flush_all_updates_stats() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Memory, "b", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Vision, "c", serde_json::json!({}));
        queue.flush_all();
        assert_eq!(queue.total_flushed(), 3);
        assert_eq!(queue.total_batches(), 2);
    }

    // ── Batch size trigger ────────────────────────────────────

    #[test]
    fn sisters_ready_when_batch_full() {
        let mut queue = BatchQueue::new(default_config()); // max_batch_size = 3
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Memory, "b", serde_json::json!({}));
        assert!(queue.sisters_ready_to_flush().is_empty());
        queue.enqueue(BatchSisterId::Memory, "c", serde_json::json!({}));
        let ready = queue.sisters_ready_to_flush();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], BatchSisterId::Memory);
    }

    #[test]
    fn sisters_not_ready_below_batch_size_before_timeout() {
        let config = BatchConfig {
            max_batch_size: 10,
            flush_timeout: Duration::from_secs(3600), // very long timeout
            overhead_per_call: 50,
        };
        let mut queue = BatchQueue::new(config);
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        assert!(queue.sisters_ready_to_flush().is_empty());
    }

    // ── Timeout trigger ───────────────────────────────────────

    #[test]
    fn sisters_ready_after_timeout() {
        let config = BatchConfig {
            max_batch_size: 100,
            flush_timeout: Duration::from_millis(1),
            overhead_per_call: 50,
        };
        let mut queue = BatchQueue::new(config);
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        std::thread::sleep(Duration::from_millis(5));
        let ready = queue.sisters_ready_to_flush();
        assert!(ready.contains(&BatchSisterId::Memory));
    }

    // ── Token savings ─────────────────────────────────────────

    #[test]
    fn tokens_saved_zero_when_no_batches() {
        let queue = BatchQueue::new(default_config());
        assert_eq!(queue.total_tokens_saved(), 0);
    }

    #[test]
    fn tokens_saved_after_batching() {
        let mut queue = BatchQueue::new(default_config());
        // Queue 3 calls for memory, flush as 1 batch
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Memory, "b", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Memory, "c", serde_json::json!({}));
        queue.flush(BatchSisterId::Memory);
        // 3 flushed - 1 batch = 2 calls saved overhead => 2 * 50 = 100
        assert_eq!(queue.total_tokens_saved(), 100);
    }

    #[test]
    fn tokens_saved_single_call_batch_saves_nothing() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        queue.flush(BatchSisterId::Memory);
        // 1 flushed - 1 batch = 0 saved
        assert_eq!(queue.total_tokens_saved(), 0);
    }

    #[test]
    fn batch_flush_result_tokens_saved() {
        let result = BatchFlushResult {
            sister_id: BatchSisterId::Memory,
            calls: vec![],
            batch_count: 1,
            individual_count: 5,
        };
        // 5 individual - 1 batch overhead = 4 * 50 = 200
        assert_eq!(result.tokens_saved(50), 200);
    }

    #[test]
    fn batch_flush_result_tokens_saved_single_call() {
        let result = BatchFlushResult {
            sister_id: BatchSisterId::Memory,
            calls: vec![],
            batch_count: 1,
            individual_count: 1,
        };
        assert_eq!(result.tokens_saved(50), 0);
    }

    // ── Stats ─────────────────────────────────────────────────

    #[test]
    fn stats_accumulate_across_flushes() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(BatchSisterId::Memory, "a", serde_json::json!({}));
        queue.flush(BatchSisterId::Memory);
        queue.enqueue(BatchSisterId::Vision, "b", serde_json::json!({}));
        queue.enqueue(BatchSisterId::Vision, "c", serde_json::json!({}));
        queue.flush(BatchSisterId::Vision);
        assert_eq!(queue.total_queued(), 3);
        assert_eq!(queue.total_flushed(), 3);
        assert_eq!(queue.total_batches(), 2);
    }

    // ── BatchSisterId ─────────────────────────────────────────

    #[test]
    fn batch_sister_id_name() {
        assert_eq!(BatchSisterId::Memory.name(), "memory");
        assert_eq!(BatchSisterId::Vision.name(), "vision");
        assert_eq!(BatchSisterId::Forge.name(), "forge");
    }

    #[test]
    fn batch_sister_id_serialization_roundtrip() {
        let id = BatchSisterId::Cognition;
        let json = serde_json::to_string(&id).unwrap();
        let restored: BatchSisterId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    // ── Call params preserved ─────────────────────────────────

    #[test]
    fn flush_preserves_call_params() {
        let mut queue = BatchQueue::new(default_config());
        queue.enqueue(
            BatchSisterId::Memory,
            "memory_add",
            serde_json::json!({"content": "hello", "tags": ["test"]}),
        );
        let result = queue.flush(BatchSisterId::Memory).unwrap();
        assert_eq!(result.calls[0].params["content"], "hello");
        assert_eq!(result.calls[0].params["tags"][0], "test");
    }
}
