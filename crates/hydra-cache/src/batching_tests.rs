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
