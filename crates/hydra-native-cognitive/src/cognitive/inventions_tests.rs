//! Tests for the InventionEngine.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::cognitive::inventions::InventionEngine;

    #[test]
    fn test_invention_engine_new() {
        let engine = InventionEngine::new();
        assert_eq!(engine.idle_time(), 0);
    }

    #[test]
    fn test_idle_tracking() {
        let engine = InventionEngine::new();
        engine.tick_idle(10);
        assert_eq!(engine.idle_time(), 10);
        engine.tick_idle(20);
        assert_eq!(engine.idle_time(), 30);
        engine.reset_idle();
        assert_eq!(engine.idle_time(), 0);
    }

    #[test]
    fn test_dream_requires_idle() {
        let engine = InventionEngine::new();
        assert!(engine.maybe_dream().is_none()); // Not idle enough
    }

    #[test]
    fn test_dream_after_idle() {
        let engine = InventionEngine::new();
        engine.tick_idle(70); // Idle enough, enters DeepIdle
        let result = engine.maybe_dream();
        assert!(result.is_some());
    }

    #[test]
    fn test_surface_insights_empty() {
        let engine = InventionEngine::new();
        assert!(engine.surface_insights(0.6).is_none());
    }

    #[test]
    fn test_shadow_validate() {
        let engine = InventionEngine::new();
        let expected = HashMap::from([
            ("shadow_output".to_string(), serde_json::json!({"test": true})),
        ]);
        let (safe, rec) = engine.shadow_validate("test action", &expected);
        assert!(safe);
        assert!(rec.contains("Shadow validation"));
    }

    #[test]
    fn test_predict_outcome() {
        let engine = InventionEngine::new();
        let (conf, rec, desc) = engine.predict_outcome("read_file", 0.1);
        assert!(conf > 0.0);
        assert!(!rec.is_empty());
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_compress_context() {
        let engine = InventionEngine::new();
        let content = "hello   world   foo   bar\n\n\n\nbaz   qux\n\n\n\n";
        let (compressed, ratio) = engine.compress_context(content);
        assert!(!compressed.is_empty());
        assert!(ratio >= 0.0);
    }

    #[test]
    fn test_record_action_tracks_pattern() {
        let engine = InventionEngine::new();
        let actions = vec!["read".to_string(), "modify".to_string(), "write".to_string()];

        // First two calls: no crystallization yet
        assert!(engine.record_action("edit_file", &actions, true, 100).is_none());
        assert!(engine.record_action("edit_file", &actions, true, 120).is_none());

        // Pattern count should be 1 (same name, re-used)
        assert_eq!(engine.pattern_count(), 1);
    }

    #[test]
    fn test_record_action_crystallizes() {
        let engine = InventionEngine::new();
        let actions = vec!["read".to_string(), "write".to_string()];

        // Record 3 successful executions to trigger crystallization
        engine.record_action("save_file", &actions, true, 50);
        engine.record_action("save_file", &actions, true, 60);
        let result = engine.record_action("save_file", &actions, true, 55);

        // Should have crystallized after 3 successful occurrences (100% success >= 70%)
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "save_file");
        assert_eq!(engine.skill_count(), 1);
    }

    #[test]
    fn test_reflect_returns_insights() {
        let engine = InventionEngine::new();

        // Record a high-confidence failure to trigger bias detection
        let insights = engine.reflect("risky action", 0.95, false);
        // First call may or may not produce insights depending on threshold
        // But reflection_count should increase
        assert!(engine.reflection_count() > 0 || insights.is_empty());
    }

    #[test]
    fn test_reflect_overconfidence_detection() {
        let engine = InventionEngine::new();

        // Record multiple high-confidence failures
        engine.reflect("action 1", 0.95, false);
        engine.reflect("action 2", 0.90, false);
        let insights = engine.reflect("action 3", 0.92, false);

        // Should detect overconfidence bias
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_evolve_patterns_empty() {
        let engine = InventionEngine::new();
        // No patterns tracked, so nothing to evolve
        assert!(engine.evolve_patterns().is_none());
    }

    #[test]
    fn test_evolve_patterns_with_data() {
        let engine = InventionEngine::new();
        let actions = vec!["step_a".to_string(), "step_b".to_string()];

        // Record enough to have a pattern
        for _ in 0..5 {
            engine.record_action("workflow", &actions, true, 100);
        }

        let result = engine.evolve_patterns();
        assert!(result.is_some());
        let summary = result.unwrap();
        assert!(summary.contains("Generation 1"));
    }

    #[test]
    fn test_store_and_recall_temporal() {
        let engine = InventionEngine::new();
        engine.store_temporal("User prefers dark mode", "preferences", 0.8);
        engine.store_temporal("Installed rust toolchain", "actions", 0.5);
        engine.store_temporal("User likes Rust", "preferences", 0.7);

        let results = engine.recall_temporal("User", 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_recall_temporal_empty() {
        let engine = InventionEngine::new();
        let results = engine.recall_temporal("nonexistent", 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_skill_count_initially_zero() {
        let engine = InventionEngine::new();
        assert_eq!(engine.skill_count(), 0);
    }

    #[test]
    fn test_pattern_count_initially_zero() {
        let engine = InventionEngine::new();
        assert_eq!(engine.pattern_count(), 0);
    }

    #[test]
    fn test_reflection_count_initially_zero() {
        let engine = InventionEngine::new();
        assert_eq!(engine.reflection_count(), 0);
    }
}
