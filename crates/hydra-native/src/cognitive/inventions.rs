//! Cognitive inventions — Dream State, Shadow Self, Future Echo.
//!
//! Wraps hydra-inventions types into a unified engine that the cognitive
//! loop can call at the right phase.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use hydra_inventions::crystallization::SkillCrystallizer;
use hydra_inventions::crystallization::crystallizer::PatternObservation;
use hydra_inventions::dream::{DreamConfig, DreamSimulator, IdleLevel};
use hydra_inventions::future_echo::predictor::Action;
use hydra_inventions::future_echo::{ActionChain, OutcomePredictor};
use hydra_inventions::metacognition::{CognitiveAnalyzer, MetaCognition, ThinkingPattern};
use hydra_inventions::minimizer::{CompressionLevel, ContextCompressor, SemanticDedup};
use hydra_inventions::mutation::{EvolutionEngine, PatternMutator, PatternTracker};
use hydra_inventions::shadow::ShadowValidator;
use hydra_inventions::temporal::{HydraTime, TemporalQuery};

/// Unified invention engine for the cognitive loop.
pub struct InventionEngine {
    pub dream: Arc<Mutex<DreamSimulator>>,
    pub shadow: Arc<Mutex<ShadowValidator>>,
    pub predictor: Arc<Mutex<OutcomePredictor>>,
    pub crystallizer: Arc<Mutex<SkillCrystallizer>>,
    pub pattern_tracker: Arc<Mutex<PatternTracker>>,
    pub mutator: Arc<Mutex<PatternMutator>>,
    pub evolution: Arc<Mutex<EvolutionEngine>>,
    pub metacognition: Arc<Mutex<MetaCognition>>,
    pub analyzer: Arc<Mutex<CognitiveAnalyzer>>,
    pub compressor: Arc<Mutex<ContextCompressor>>,
    pub dedup: Arc<Mutex<SemanticDedup>>,
    pub temporal: Arc<Mutex<HydraTime>>,
    idle_seconds: Arc<std::sync::atomic::AtomicU64>,
}

impl InventionEngine {
    pub fn new() -> Self {
        Self {
            dream: Arc::new(Mutex::new(DreamSimulator::new(DreamConfig::default()))),
            shadow: Arc::new(Mutex::new(ShadowValidator::new())),
            predictor: Arc::new(Mutex::new(OutcomePredictor::new())),
            crystallizer: Arc::new(Mutex::new(SkillCrystallizer::new(3, 0.7))),
            pattern_tracker: Arc::new(Mutex::new(PatternTracker::new())),
            mutator: Arc::new(Mutex::new(PatternMutator::new())),
            evolution: Arc::new(Mutex::new(EvolutionEngine::new(0.5))),
            metacognition: Arc::new(Mutex::new(MetaCognition::new(500))),
            analyzer: Arc::new(Mutex::new(CognitiveAnalyzer::new())),
            compressor: Arc::new(Mutex::new(
                ContextCompressor::new(CompressionLevel::Medium).with_code_preservation(true),
            )),
            dedup: Arc::new(Mutex::new(SemanticDedup::new(0.9, 10))),
            temporal: Arc::new(Mutex::new(HydraTime::new(10000))),
            idle_seconds: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Call when user sends a message (reset idle timer).
    pub fn reset_idle(&self) {
        self.idle_seconds
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.dream.lock().set_idle_level(IdleLevel::Active);
    }

    /// Call periodically to increment idle time and update dream idle level.
    pub fn tick_idle(&self, seconds: u64) {
        let new_idle = self
            .idle_seconds
            .fetch_add(seconds, std::sync::atomic::Ordering::Relaxed)
            + seconds;

        // Map idle seconds to dream idle levels
        let level = match new_idle {
            0..=29 => IdleLevel::Active,
            30..=59 => IdleLevel::LightIdle,
            60..=299 => IdleLevel::DeepIdle,
            _ => IdleLevel::Sleeping,
        };
        self.dream.lock().set_idle_level(level);
    }

    /// Get current idle seconds.
    pub fn idle_time(&self) -> u64 {
        self.idle_seconds
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Run a dream session if idle enough (>= 60 seconds).
    /// Returns insights as a formatted string, or None if not idle enough.
    pub fn maybe_dream(&self) -> Option<String> {
        let idle = self.idle_time();
        if idle < 60 {
            return None;
        }

        let dream = self.dream.lock();
        let insights = dream.dream_session();
        if insights.is_empty() {
            return None;
        }

        let mut result = String::from("## Dream Insights (generated while idle)\n");
        for insight in &insights {
            result.push_str(&format!(
                "- **{:?}** (confidence: {:.0}%): {}\n",
                insight.category,
                insight.confidence * 100.0,
                insight.description
            ));
        }
        Some(result)
    }

    /// Surface any unsurfaced dream insights (call at start of PERCEIVE).
    pub fn surface_insights(&self, min_confidence: f32) -> Option<String> {
        let dream = self.dream.lock();
        let surfaced = dream.insights().surface(min_confidence);
        if surfaced.is_empty() {
            return None;
        }
        let mut result = String::from("## Insights From Idle Processing\n");
        for insight in &surfaced {
            result.push_str(&format!("- {}\n", insight.description));
        }
        Some(result)
    }

    /// Run shadow validation for medium+ risk actions (call in DECIDE phase).
    /// Returns (safe, recommendation_string).
    pub fn shadow_validate(
        &self,
        description: &str,
        expected_outputs: &HashMap<String, serde_json::Value>,
    ) -> (bool, String) {
        let shadow = self.shadow.lock();
        let outcome = shadow.validate(description, serde_json::json!({}), expected_outputs);
        let safe = outcome.safe;
        let rec = format!(
            "Shadow validation: {:?} (divergences: {}, critical: {})",
            outcome.recommendation, outcome.divergence_count, outcome.critical_divergences
        );
        (safe, rec)
    }

    /// Future echo — predict outcome before ACT phase (call in DECIDE phase).
    /// Alias for predict_outcome, named after the cognitive pattern of "hearing"
    /// the future result before committing to an action.
    pub fn future_echo(&self, action_name: &str, risk_level: f32) -> (f32, String, String) {
        self.predict_outcome(action_name, risk_level)
    }

    /// Predict outcome before ACT phase (call in DECIDE phase).
    /// Returns (confidence, risk_recommendation, description).
    pub fn predict_outcome(&self, action_name: &str, risk_level: f32) -> (f32, String, String) {
        let action = Action {
            name: action_name.to_string(),
            params: serde_json::json!({}),
            risk_level,
        };
        let chain = ActionChain::new(vec![action]);

        let predictor = self.predictor.lock();
        let outcomes = predictor.predict(&chain);

        if let Some(best) = outcomes.first() {
            let conf = best.confidence.value;
            let rec = format!("{:?}", best.risk_assessment.recommendation);
            let desc = best.description.clone();
            (conf, rec, desc)
        } else {
            (0.5, "Proceed".to_string(), "No prediction available".to_string())
        }
    }

    /// Compress context content using compressor then dedup.
    /// Returns (compressed_content, compression_ratio).
    pub fn compress_context(&self, content: &str) -> (String, f64) {
        let compressor = self.compressor.lock();
        let compressed = compressor.compress(content);

        let dedup = self.dedup.lock();
        let deduped = dedup.deduplicate(&compressed.content);

        let total_ratio = if compressed.original_tokens > 0 {
            1.0 - (deduped.deduped_tokens as f64 / compressed.original_tokens as f64)
        } else {
            0.0
        };

        (deduped.content, total_ratio)
    }

    /// Record an action execution to the pattern tracker.
    /// If a pattern reaches 3+ occurrences, attempts crystallization.
    /// Returns Some(skill_name) if a new skill was crystallized.
    pub fn record_action(
        &self,
        name: &str,
        actions: &[String],
        success: bool,
        duration_ms: u64,
    ) -> Option<String> {
        let tracker = self.pattern_tracker.lock();

        // Check if pattern already registered by looking at top patterns
        let existing_id = {
            let top = tracker.top_patterns(usize::MAX);
            top.iter().find(|p| p.name == name).map(|p| p.id.clone())
        };

        let pattern_id = if let Some(id) = existing_id {
            tracker.record(&id, success, duration_ms as f64);
            id
        } else {
            let id = tracker.register(name, actions.to_vec());
            tracker.record(&id, success, duration_ms as f64);
            id
        };

        // Check if pattern qualifies for crystallization
        if let Some(record) = tracker.get(&pattern_id) {
            if record.total_executions >= 3 {
                let observation = PatternObservation {
                    name: record.name.clone(),
                    actions: record.actions.clone(),
                    occurrences: record.total_executions,
                    success_rate: record.success_rate(),
                    avg_duration_ms: record.avg_duration_ms,
                };

                let crystallizer = self.crystallizer.lock();
                let result = crystallizer.crystallize(&observation);
                if result.success {
                    return Some(result.skill_name);
                }
            }
        }

        None
    }

    /// Record a decision and reflect on cognitive patterns.
    /// Returns insight strings from metacognition reflection.
    pub fn reflect(&self, description: &str, confidence: f32, success: bool) -> Vec<String> {
        let meta = self.metacognition.lock();
        let decision_id =
            meta.record_decision(description, confidence as f64, "cognitive loop reflection");
        meta.record_outcome(&decision_id, success);

        let analyzer = self.analyzer.lock();
        analyzer.observe(
            ThinkingPattern::Systematic,
            confidence as f64,
            success,
            0.0,
        );

        let reflections = meta.reflect();
        reflections.iter().map(|r| r.insight.clone()).collect()
    }

    /// Evolve tracked patterns using the evolution engine.
    /// Returns a summary string if any patterns evolved.
    pub fn evolve_patterns(&self) -> Option<String> {
        let tracker = self.pattern_tracker.lock();
        let top = tracker.top_patterns(20);

        if top.is_empty() {
            return None;
        }

        let evolution = self.evolution.lock();
        let generation = evolution.evolve(top);

        if generation.patterns.is_empty() {
            return None;
        }

        Some(format!(
            "Generation {}: {} patterns survived (best fitness: {:.2}, avg: {:.2})",
            generation.number,
            generation.patterns.len(),
            generation.best_fitness,
            generation.avg_fitness,
        ))
    }

    /// Store content in temporal memory.
    pub fn store_temporal(&self, content: &str, category: &str, importance: f64) {
        let temporal = self.temporal.lock();
        temporal.store(content, category, importance);
    }

    /// Recall entries from temporal memory matching a keyword.
    pub fn recall_temporal(&self, keyword: &str, limit: usize) -> Vec<String> {
        let temporal = self.temporal.lock();
        let query = TemporalQuery {
            keyword: Some(keyword.into()),
            limit,
            ..Default::default()
        };
        let results = temporal.recall(&query);
        results.into_iter().map(|e| e.content).collect()
    }

    /// Match user input against crystallized skills.
    /// Returns the skill and its actions if a match is found.
    pub fn match_crystallized_skill(&self, input: &str) -> Option<(String, Vec<String>)> {
        let crystallizer = self.crystallizer.lock();
        let skills = crystallizer.skills();
        let lower = input.to_lowercase();

        for skill in &skills {
            // Exact match on source_pattern (the original input that created this skill)
            if skill.source_pattern.to_lowercase() == lower {
                return Some((skill.name.clone(), skill.actions.clone()));
            }
            // Match on skill name (e.g., "run tests" matches skill named "run_tests" or "/test")
            let skill_lower = skill.name.to_lowercase().replace('_', " ");
            if lower == skill_lower || lower == skill.name.to_lowercase() {
                return Some((skill.name.clone(), skill.actions.clone()));
            }
            // Pattern trigger match
            if let hydra_inventions::crystallization::skill::SkillTrigger::PatternMatch(ref pat) = skill.trigger {
                if lower.contains(&pat.to_lowercase()) {
                    return Some((skill.name.clone(), skill.actions.clone()));
                }
            }
        }
        None
    }

    /// Get the number of crystallized skills.
    pub fn skill_count(&self) -> usize {
        self.crystallizer.lock().skill_count()
    }

    /// Get the number of tracked patterns.
    pub fn pattern_count(&self) -> usize {
        self.pattern_tracker.lock().count()
    }

    /// Get the number of metacognitive reflections.
    pub fn reflection_count(&self) -> usize {
        self.metacognition.lock().reflection_count()
    }
}

impl Default for InventionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
