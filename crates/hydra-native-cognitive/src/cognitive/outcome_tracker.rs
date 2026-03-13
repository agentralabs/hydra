//! Outcome tracking — learn from interaction results to improve future performance.
//!
//! Phase 3 of the superintelligence plan. Tracks success/failure/correction
//! outcomes per intent category and topic, enabling competence-aware model
//! routing and belief confidence calibration.

use std::collections::{HashMap, VecDeque};
use crate::cognitive::intent_router::IntentCategory;

/// Maximum interactions to keep in rolling history.
const MAX_HISTORY: usize = 500;
/// Minimum interactions before judging a category.
const MIN_INTERACTIONS: u64 = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Success,
    Correction,
    Failure,
    Repeat,
    Neutral,
}

impl Outcome {
    /// Stable string for DB storage.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Correction => "correction",
            Self::Failure => "failure",
            Self::Repeat => "repeat",
            Self::Neutral => "neutral",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InteractionOutcome {
    pub intent_category: IntentCategory,
    pub topic: String,
    pub model_used: String,
    pub outcome: Outcome,
    pub tokens_used: u64,
    pub timestamp: i64, // unix epoch seconds
}

#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    pub total: u64,
    pub successes: u64,
    pub corrections: u64,
    pub failures: u64,
    pub avg_tokens: u64,
}

impl CategoryStats {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 { return 0.5; } // prior
        self.successes as f64 / self.total as f64
    }
}

#[derive(Debug, Clone, Default)]
pub struct TopicStats {
    pub total: u64,
    pub successes: u64,
    pub last_outcome: Option<Outcome>,
    pub best_model: Option<String>,
    pub model_successes: HashMap<String, u64>,
}

impl TopicStats {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 { return 0.5; }
        self.successes as f64 / self.total as f64
    }
}

#[derive(Debug)]
pub struct OutcomeTracker {
    history: VecDeque<InteractionOutcome>,
    category_stats: HashMap<String, CategoryStats>, // use category debug string as key
    topic_stats: HashMap<String, TopicStats>,
    total_interactions: u64,
    total_tokens: u64,
}

impl Default for OutcomeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl OutcomeTracker {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_HISTORY),
            category_stats: HashMap::new(),
            topic_stats: HashMap::new(),
            total_interactions: 0,
            total_tokens: 0,
        }
    }

    /// Detect outcome from user's follow-up message and exec results.
    pub fn detect_outcome(
        &self,
        _previous_response: &str,
        current_input: &str,
        exec_results: &[(String, String, bool)],
    ) -> Outcome {
        let input_lower = current_input.to_lowercase();

        // Correction signals
        let correction_words = ["no,", "wrong", "actually", "that's not", "incorrect",
            "not right", "nope", "no that", "fix that", "that's wrong"];
        for word in &correction_words {
            if input_lower.contains(word) { return Outcome::Correction; }
        }

        // Success signals
        let success_words = ["thanks", "perfect", "great", "awesome", "exactly",
            "that works", "nice", "good job", "well done", "correct"];
        for word in &success_words {
            if input_lower.contains(word) { return Outcome::Success; }
        }

        // Failure from exec results
        let has_failures = exec_results.iter().any(|(_, _, success)| !success);
        let has_successes = exec_results.iter().any(|(_, _, success)| *success);
        if has_failures && !has_successes { return Outcome::Failure; }
        if has_successes && !has_failures { return Outcome::Success; }

        // Undo/revert signals
        if input_lower.contains("undo") || input_lower.contains("revert") {
            return Outcome::Failure;
        }

        Outcome::Neutral
    }

    /// Record an outcome and update all stats.
    pub fn record(
        &mut self,
        category: IntentCategory,
        topic: &str,
        model: &str,
        outcome: Outcome,
        tokens: u64,
    ) {
        let now = chrono::Utc::now().timestamp();
        let interaction = InteractionOutcome {
            intent_category: category,
            topic: topic.to_string(),
            model_used: model.to_string(),
            outcome: outcome.clone(),
            tokens_used: tokens,
            timestamp: now,
        };

        // Add to history (evict old if full)
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(interaction);

        // Update category stats
        let cat_key = format!("{:?}", category);
        let cat = self.category_stats.entry(cat_key).or_default();
        cat.total += 1;
        match &outcome {
            Outcome::Success => cat.successes += 1,
            Outcome::Correction => cat.corrections += 1,
            Outcome::Failure => cat.failures += 1,
            _ => {}
        }
        // Running average of tokens
        cat.avg_tokens = (cat.avg_tokens * (cat.total - 1) + tokens) / cat.total;

        // Update topic stats
        if !topic.is_empty() {
            let ts = self.topic_stats.entry(topic.to_string()).or_default();
            ts.total += 1;
            if outcome == Outcome::Success {
                ts.successes += 1;
                *ts.model_successes.entry(model.to_string()).or_insert(0) += 1;
                // Update best model
                let best = ts.model_successes.iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(m, _)| m.clone());
                ts.best_model = best;
            }
            ts.last_outcome = Some(outcome);
        }

        self.total_interactions += 1;
        self.total_tokens += tokens;
    }

    /// Get success rate for a category.
    pub fn category_success_rate(&self, cat: IntentCategory) -> f64 {
        let key = format!("{:?}", cat);
        self.category_stats.get(&key).map(|s| s.success_rate()).unwrap_or(0.5)
    }

    /// Get best model for a topic.
    pub fn best_model_for_topic(&self, topic: &str) -> Option<String> {
        self.topic_stats.get(topic).and_then(|s| s.best_model.clone())
    }

    /// Suggest confidence adjustment based on historical performance.
    /// Returns a multiplier (0.5 to 1.2) for the LLM's confidence.
    pub fn confidence_adjustment(&self, cat: IntentCategory, topic: &str) -> f64 {
        let cat_rate = self.category_success_rate(cat);
        let topic_rate = self.topic_stats.get(topic)
            .map(|s| s.success_rate()).unwrap_or(0.5);
        // Blend category and topic rates
        let blended = cat_rate * 0.6 + topic_rate * 0.4;
        // Map to multiplier: 0% success -> 0.5x, 50% -> 1.0x, 100% -> 1.2x
        0.5 + blended * 0.7
    }

    /// Get total interactions tracked.
    pub fn total_interactions(&self) -> u64 { self.total_interactions }

    /// Identify weak categories (success rate below threshold with enough data).
    pub fn weak_categories(&self, max_success_rate: f64) -> Vec<(String, f64)> {
        self.category_stats.iter()
            .filter(|(_, stats)| {
                stats.total >= MIN_INTERACTIONS
                    && stats.success_rate() < max_success_rate
            })
            .map(|(cat, stats)| (cat.clone(), stats.success_rate()))
            .collect()
    }

    /// Summary for display.
    pub fn summary(&self) -> String {
        let mut out = format!(
            "Interactions: {}, Tokens: {}\n",
            self.total_interactions, self.total_tokens,
        );
        let mut cats: Vec<_> = self.category_stats.iter().collect();
        cats.sort_by(|a, b| a.0.cmp(b.0));
        for (cat, stats) in cats {
            out.push_str(&format!(
                "  {}: {:.0}% ({} total, {} corrections)\n",
                cat, stats.success_rate() * 100.0, stats.total, stats.corrections,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_correction() {
        let tracker = OutcomeTracker::new();
        assert_eq!(
            tracker.detect_outcome("response", "no, that's wrong", &[]),
            Outcome::Correction,
        );
        assert_eq!(
            tracker.detect_outcome("response", "actually it should be X", &[]),
            Outcome::Correction,
        );
    }

    #[test]
    fn test_detect_success() {
        let tracker = OutcomeTracker::new();
        assert_eq!(
            tracker.detect_outcome("response", "thanks, that's perfect", &[]),
            Outcome::Success,
        );
        assert_eq!(
            tracker.detect_outcome("response", "great job!", &[]),
            Outcome::Success,
        );
    }

    #[test]
    fn test_detect_from_exec_results() {
        let tracker = OutcomeTracker::new();
        let ok_results = vec![("cmd".into(), "output".into(), true)];
        assert_eq!(
            tracker.detect_outcome("r", "next thing", &ok_results),
            Outcome::Success,
        );
        let fail_results = vec![("cmd".into(), "error".into(), false)];
        assert_eq!(
            tracker.detect_outcome("r", "next thing", &fail_results),
            Outcome::Failure,
        );
    }

    #[test]
    fn test_record_and_stats() {
        let mut tracker = OutcomeTracker::new();
        for _ in 0..8 {
            tracker.record(IntentCategory::CodeBuild, "rust", "sonnet", Outcome::Success, 1000);
        }
        for _ in 0..2 {
            tracker.record(IntentCategory::CodeBuild, "rust", "sonnet", Outcome::Failure, 1000);
        }
        let rate = tracker.category_success_rate(IntentCategory::CodeBuild);
        assert!((rate - 0.8).abs() < 0.01);
        assert_eq!(tracker.total_interactions(), 10);
    }

    #[test]
    fn test_best_model_for_topic() {
        let mut tracker = OutcomeTracker::new();
        tracker.record(IntentCategory::CodeBuild, "async_rust", "haiku", Outcome::Failure, 500);
        tracker.record(IntentCategory::CodeBuild, "async_rust", "sonnet", Outcome::Success, 1500);
        tracker.record(IntentCategory::CodeBuild, "async_rust", "sonnet", Outcome::Success, 1500);
        assert_eq!(
            tracker.best_model_for_topic("async_rust"),
            Some("sonnet".to_string()),
        );
    }

    #[test]
    fn test_weak_categories() {
        let mut tracker = OutcomeTracker::new();
        for _ in 0..12 {
            tracker.record(IntentCategory::Deploy, "docker", "haiku", Outcome::Failure, 500);
        }
        let weak = tracker.weak_categories(0.6);
        assert!(!weak.is_empty());
        assert!(weak[0].1 < 0.1); // 0% success rate
    }

    #[test]
    fn test_confidence_adjustment() {
        let mut tracker = OutcomeTracker::new();
        // Perfect success should give multiplier > 1.0
        for _ in 0..20 {
            tracker.record(IntentCategory::CodeBuild, "rust", "sonnet", Outcome::Success, 1000);
        }
        let adj = tracker.confidence_adjustment(IntentCategory::CodeBuild, "rust");
        assert!(adj > 1.0);

        // Total failure should give multiplier < 1.0
        let mut tracker2 = OutcomeTracker::new();
        for _ in 0..20 {
            tracker2.record(IntentCategory::Deploy, "k8s", "haiku", Outcome::Failure, 500);
        }
        let adj2 = tracker2.confidence_adjustment(IntentCategory::Deploy, "k8s");
        assert!(adj2 < 1.0);
    }

    #[test]
    fn test_neutral_no_signal() {
        let tracker = OutcomeTracker::new();
        assert_eq!(
            tracker.detect_outcome("response", "ok let me try something else", &[]),
            Outcome::Neutral,
        );
    }
}
