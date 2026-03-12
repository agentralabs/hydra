#[cfg(test)]
mod tests {
    use crate::intent_cache::*;
    use hydra_core::types::*;
    use std::time::Duration;

    /// Helper: build a minimal CompiledIntent for testing
    fn make_intent(text: &str, confidence: f64) -> CompiledIntent {
        CompiledIntent {
            id: uuid::Uuid::new_v4(),
            raw_text: text.to_string(),
            source: IntentSource::Cli,
            goal: Goal {
                goal_type: GoalType::Query,
                target: "test".to_string(),
                outcome: "test outcome".to_string(),
                sub_goals: vec![],
            },
            entities: vec![],
            actions: vec![],
            constraints: vec![],
            success_criteria: vec![],
            confidence,
            estimated_steps: 1,
            tokens_used: 0,
            veritas_validation: VeritasValidation {
                validated: true,
                safety_score: 1.0,
                warnings: vec![],
            },
        }
    }

    // ── Basic get/put ─────────────────────────────────────────

    #[test]
    fn cache_miss_on_empty_cache() {
        let cache = IntentCache::without_ttl(100);
        assert!(cache.get("hello world").is_none());
        assert_eq!(cache.total_misses(), 1);
        assert_eq!(cache.total_hits(), 0);
    }

    #[test]
    fn cache_hit_after_put() {
        let cache = IntentCache::without_ttl(100);
        let intent = make_intent("run tests", 0.95);
        cache.put("run tests", intent.clone());
        let result = cache.get("run tests");
        assert!(result.is_some());
        assert_eq!(result.unwrap().raw_text, "run tests");
        assert_eq!(cache.total_hits(), 1);
    }

    #[test]
    fn put_overwrites_existing_entry() {
        let cache = IntentCache::without_ttl(100);
        cache.put("run tests", make_intent("run tests", 0.5));
        cache.put("run tests", make_intent("run tests", 0.99));
        let result = cache.get("run tests").unwrap();
        assert!((result.confidence - 0.99).abs() < f64::EPSILON);
    }

    #[test]
    fn cache_returns_zero_tokens_used_for_cached_intent() {
        let cache = IntentCache::without_ttl(100);
        let intent = make_intent("deploy app", 0.9);
        assert_eq!(intent.tokens_used, 0);
        cache.put("deploy app", intent);
        let cached = cache.get("deploy app").unwrap();
        assert_eq!(cached.tokens_used, 0, "cached path should use 0 tokens");
    }

    // ── Key normalization ─────────────────────────────────────

    #[test]
    fn cache_key_normalizes_whitespace() {
        let cache = IntentCache::without_ttl(100);
        cache.put("run   the   tests", make_intent("run the tests", 0.9));
        assert!(cache.get("run the tests").is_some());
    }

    #[test]
    fn cache_key_normalizes_case() {
        let cache = IntentCache::without_ttl(100);
        cache.put("Run Tests", make_intent("run tests", 0.9));
        assert!(cache.get("run tests").is_some());
        assert!(cache.get("RUN TESTS").is_some());
    }

    #[test]
    fn cache_key_normalizes_leading_trailing_whitespace() {
        let cache = IntentCache::without_ttl(100);
        cache.put("  run tests  ", make_intent("run tests", 0.9));
        assert!(cache.get("run tests").is_some());
    }

    #[test]
    fn cache_key_with_context_differs_from_without() {
        let cache = IntentCache::without_ttl(100);
        cache.put("run tests", make_intent("run tests", 0.8));
        cache.put_with_context("run tests", Some(42), make_intent("run tests ctx", 0.95));
        // Without context: gets the first one
        let no_ctx = cache.get("run tests").unwrap();
        assert!((no_ctx.confidence - 0.8).abs() < f64::EPSILON);
        // With context: gets the second one
        let with_ctx = cache.get_with_context("run tests", Some(42)).unwrap();
        assert!((with_ctx.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn different_context_hashes_are_different_entries() {
        let cache = IntentCache::without_ttl(100);
        cache.put_with_context("deploy", Some(1), make_intent("deploy", 0.7));
        cache.put_with_context("deploy", Some(2), make_intent("deploy", 0.9));
        assert_eq!(cache.len(), 2);
        let r1 = cache.get_with_context("deploy", Some(1)).unwrap();
        let r2 = cache.get_with_context("deploy", Some(2)).unwrap();
        assert!((r1.confidence - 0.7).abs() < f64::EPSILON);
        assert!((r2.confidence - 0.9).abs() < f64::EPSILON);
    }

    // ── TTL expiry ────────────────────────────────────────────

    #[test]
    fn ttl_zero_expires_immediately() {
        let cache = IntentCache::new(100, Duration::from_nanos(0));
        cache.put("hello", make_intent("hello", 0.9));
        // Even a nanosecond TTL should expire by the time we call get
        std::thread::sleep(Duration::from_millis(1));
        assert!(cache.get("hello").is_none());
        assert_eq!(cache.total_evictions(), 1);
    }

    #[test]
    fn ttl_long_does_not_expire() {
        let cache = IntentCache::new(100, Duration::from_secs(3600));
        cache.put("hello", make_intent("hello", 0.9));
        assert!(cache.get("hello").is_some());
    }

    #[test]
    fn expired_entry_counts_as_miss() {
        let cache = IntentCache::new(100, Duration::from_nanos(0));
        cache.put("test", make_intent("test", 0.9));
        std::thread::sleep(Duration::from_millis(1));
        cache.get("test");
        assert_eq!(cache.total_hits(), 0);
        assert_eq!(cache.total_misses(), 1);
    }

    #[test]
    fn purge_expired_removes_old_entries() {
        let cache = IntentCache::new(100, Duration::from_nanos(0));
        cache.put("a", make_intent("a", 0.9));
        cache.put("b", make_intent("b", 0.9));
        cache.put("c", make_intent("c", 0.9));
        std::thread::sleep(Duration::from_millis(1));
        let purged = cache.purge_expired();
        assert_eq!(purged, 3);
        assert!(cache.is_empty());
    }

    #[test]
    fn purge_expired_keeps_fresh_entries() {
        let cache = IntentCache::new(100, Duration::from_secs(3600));
        cache.put("fresh", make_intent("fresh", 0.9));
        let purged = cache.purge_expired();
        assert_eq!(purged, 0);
        assert_eq!(cache.len(), 1);
    }

    // ── Capacity / Eviction ───────────────────────────────────

    #[test]
    fn eviction_at_capacity() {
        let cache = IntentCache::without_ttl(2);
        cache.put("first", make_intent("first", 0.9));
        cache.put("second", make_intent("second", 0.9));
        assert_eq!(cache.len(), 2);
        cache.put("third", make_intent("third", 0.9));
        // Should still be at capacity (one was evicted)
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.total_evictions(), 1);
    }

    #[test]
    fn eviction_prefers_least_accessed() {
        let cache = IntentCache::without_ttl(2);
        cache.put("popular", make_intent("popular", 0.9));
        cache.put("unpopular", make_intent("unpopular", 0.9));
        // Access "popular" several times to increase its access_count
        for _ in 0..5 {
            cache.get("popular");
        }
        // Now insert a third entry — "unpopular" should be evicted
        cache.put("newcomer", make_intent("newcomer", 0.9));
        assert!(cache.get("popular").is_some(), "popular should survive eviction");
        assert!(cache.get("newcomer").is_some(), "newcomer should exist");
    }

    #[test]
    fn capacity_one_cache_always_has_one_entry() {
        let cache = IntentCache::without_ttl(1);
        cache.put("a", make_intent("a", 0.9));
        assert_eq!(cache.len(), 1);
        cache.put("b", make_intent("b", 0.9));
        assert_eq!(cache.len(), 1);
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
    }

    // ── Invalidation ──────────────────────────────────────────

    #[test]
    fn invalidate_removes_entry() {
        let cache = IntentCache::without_ttl(100);
        cache.put("run tests", make_intent("run tests", 0.9));
        cache.invalidate("run tests");
        assert!(cache.get("run tests").is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn invalidate_nonexistent_is_noop() {
        let cache = IntentCache::without_ttl(100);
        cache.invalidate("nothing");
        assert!(cache.is_empty());
    }

    #[test]
    fn invalidate_with_context_only_removes_that_entry() {
        let cache = IntentCache::without_ttl(100);
        cache.put("deploy", make_intent("deploy", 0.8));
        cache.put_with_context("deploy", Some(42), make_intent("deploy", 0.95));
        cache.invalidate_with_context("deploy", Some(42));
        assert!(cache.get("deploy").is_some());
        assert!(cache.get_with_context("deploy", Some(42)).is_none());
    }

    #[test]
    fn clear_removes_everything() {
        let cache = IntentCache::without_ttl(100);
        cache.put("a", make_intent("a", 0.9));
        cache.put("b", make_intent("b", 0.9));
        cache.put("c", make_intent("c", 0.9));
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    // ── Statistics ────────────────────────────────────────────

    #[test]
    fn hit_rate_zero_when_empty() {
        let cache = IntentCache::without_ttl(100);
        assert!((cache.hit_rate() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hit_rate_one_when_all_hits() {
        let cache = IntentCache::without_ttl(100);
        cache.put("x", make_intent("x", 0.9));
        cache.get("x");
        cache.get("x");
        cache.get("x");
        assert!((cache.hit_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn hit_rate_half_with_equal_hits_and_misses() {
        let cache = IntentCache::without_ttl(100);
        cache.put("x", make_intent("x", 0.9));
        cache.get("x"); // hit
        cache.get("y"); // miss
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn tokens_saved_scales_with_hits() {
        let cache = IntentCache::without_ttl(100);
        cache.put("x", make_intent("x", 0.9));
        cache.get("x");
        cache.get("x");
        cache.get("x");
        assert_eq!(cache.tokens_saved(), 600); // 3 hits * 200
    }

    #[test]
    fn tokens_saved_zero_on_no_hits() {
        let cache = IntentCache::without_ttl(100);
        cache.get("nonexistent");
        assert_eq!(cache.tokens_saved(), 0);
    }

    #[test]
    fn capacity_returns_configured_max() {
        let cache = IntentCache::without_ttl(42);
        assert_eq!(cache.capacity(), 42);
    }
}
