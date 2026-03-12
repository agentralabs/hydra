//! Cognitive inventions — session momentum and intelligence upgrade methods.

use super::inventions_core::InventionEngine;

impl InventionEngine {
    // ═══════════════════════════════════════════════════════════════
    // Phase 2, L3: Session Momentum Tracking
    // ═══════════════════════════════════════════════════════════════

    /// Record a successful interaction for session momentum tracking.
    pub fn record_session_success(&self) {
        self.session_successes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a failed interaction for session momentum tracking.
    pub fn record_session_failure(&self) {
        self.session_failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a correction for session momentum tracking.
    pub fn record_session_correction(&self) {
        self.session_corrections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get session momentum: (successes, failures, corrections).
    pub fn session_momentum(&self) -> (u64, u64, u64) {
        (
            self.session_successes.load(std::sync::atomic::Ordering::Relaxed),
            self.session_failures.load(std::sync::atomic::Ordering::Relaxed),
            self.session_corrections.load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Check if session momentum indicates Hydra should be more cautious.
    /// Returns true if 3+ corrections have occurred this session.
    pub fn should_be_cautious(&self) -> bool {
        let corrections = self.session_corrections.load(std::sync::atomic::Ordering::Relaxed);
        corrections >= 3
    }

    /// Get a confidence penalty based on session momentum.
    /// More corrections = larger penalty. Max penalty is 0.3 (30%).
    pub fn momentum_confidence_penalty(&self) -> f64 {
        let corrections = self.session_corrections.load(std::sync::atomic::Ordering::Relaxed);
        let failures = self.session_failures.load(std::sync::atomic::Ordering::Relaxed);
        let penalty = (corrections as f64 * 0.08) + (failures as f64 * 0.03);
        penalty.min(0.3)
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 1 Intelligence Upgrade — New helper methods
    // ═══════════════════════════════════════════════════════════════

    /// Recall recent temporal memory entries related to the user's input.
    /// Used in THINK phase to inject recent interaction context.
    /// Returns formatted context string, or None if nothing relevant found.
    pub fn recall_temporal_context(&self, input: &str, limit: usize) -> Option<String> {
        // Extract keywords from input (words >= 4 chars, skip stop words)
        let stop_words = ["what", "that", "this", "with", "from", "have", "been",
            "will", "would", "could", "should", "about", "where", "when", "which",
            "their", "there", "these", "those", "than", "then", "into", "some",
            "your", "they", "them", "does", "make", "like", "just"];
        let keywords: Vec<&str> = input.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.len() >= 4 && !stop_words.contains(&w.to_lowercase().as_str()))
            .collect();

        if keywords.is_empty() {
            return None;
        }

        let mut all_results = Vec::new();
        for kw in &keywords {
            let results = self.recall_temporal(kw, limit);
            for r in results {
                if !all_results.contains(&r) {
                    all_results.push(r);
                }
            }
            if all_results.len() >= limit {
                break;
            }
        }

        if all_results.is_empty() {
            return None;
        }

        all_results.truncate(limit);
        let mut context = String::from("Recent related interactions:\n");
        for entry in &all_results {
            context.push_str(&format!("- {}\n", entry));
        }
        Some(context)
    }

    /// Check if metacognition detects overconfidence bias.
    /// Returns true if the system has been overconfident recently,
    /// along with a recommended confidence adjustment factor (0.0-1.0).
    pub fn check_overconfidence(&self) -> (bool, f32) {
        let meta = self.metacognition.lock();

        // Check reflections for overconfidence signals
        let reflections = meta.reflect();
        let has_overconfidence = reflections.iter()
            .any(|r| r.insight.to_lowercase().contains("overconfiden"));

        if has_overconfidence {
            // Suggest reducing confidence by 20%
            (true, 0.8)
        } else if meta.reflection_count() > 5 {
            // Many reflections without overconfidence = generally fine
            (false, 1.0)
        } else {
            (false, 1.0)
        }
    }

    /// Get prediction confidence adjustment based on historical outcomes
    /// for similar actions. Returns adjusted confidence multiplier.
    pub fn historical_confidence_for(&self, action_name: &str) -> f32 {
        let tracker = self.pattern_tracker.lock();
        let top = tracker.top_patterns(50);

        // Look for patterns with similar names
        let lower = action_name.to_lowercase();
        for pattern in &top {
            if pattern.name.to_lowercase().contains(&lower)
                || lower.contains(&pattern.name.to_lowercase())
            {
                // Use historical success rate as confidence multiplier
                return pattern.success_rate() as f32;
            }
        }

        // No historical data — return neutral
        1.0
    }
}
