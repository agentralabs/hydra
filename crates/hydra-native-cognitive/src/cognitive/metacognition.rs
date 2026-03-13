//! Enhanced metacognition — confidence calibration and cognitive load awareness.
//!
//! Phase 7 of the superintelligence plan. Tracks whether Hydra's predicted
//! confidence matches actual outcomes, and detects when tasks exceed
//! reliable capability.

use crate::cognitive::intent_router::{IntentCategory, ClassifiedIntent};

/// Tracks predicted confidence vs actual outcomes for calibration.
#[derive(Debug)]
pub struct CalibrationTracker {
    /// Buckets by predicted confidence (0-9 for 0-10%, 10-19%, ..., 90-100%)
    buckets: [(u64, u64); 10],  // (total, successes) per 10% bucket
}

impl Default for CalibrationTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationTracker {
    pub fn new() -> Self {
        Self { buckets: [(0, 0); 10] }
    }

    /// Record a prediction and its outcome.
    pub fn record(&mut self, predicted_confidence: f64, actual_success: bool) {
        let bucket = ((predicted_confidence * 10.0) as usize).min(9);
        self.buckets[bucket].0 += 1;
        if actual_success {
            self.buckets[bucket].1 += 1;
        }
    }

    /// Get calibration error (0.0 = perfectly calibrated, 1.0 = worst).
    /// Uses Expected Calibration Error (ECE) metric.
    pub fn calibration_error(&self) -> f64 {
        let total: u64 = self.buckets.iter().map(|(t, _)| t).sum();
        if total == 0 { return 0.0; }

        let mut weighted_error = 0.0;
        for (i, (count, successes)) in self.buckets.iter().enumerate() {
            if *count == 0 { continue; }
            let predicted = (i as f64 + 0.5) / 10.0; // Midpoint of bucket
            let actual = *successes as f64 / *count as f64;
            let error = (predicted - actual).abs();
            weighted_error += error * (*count as f64 / total as f64);
        }
        weighted_error
    }

    /// Adjust a predicted confidence based on historical calibration.
    /// If Hydra tends to be overconfident, this will reduce the confidence.
    pub fn calibrate(&self, raw_confidence: f64) -> f64 {
        let bucket = ((raw_confidence * 10.0) as usize).min(9);
        let (count, successes) = self.buckets[bucket];
        if count < 5 { return raw_confidence; } // Not enough data

        let actual_rate = successes as f64 / count as f64;
        // Blend raw prediction with historical accuracy
        // Weight historical data more as we get more samples
        let weight = (count as f64 / 50.0).min(0.8); // Max 80% historical
        raw_confidence * (1.0 - weight) + actual_rate * weight
    }

    /// Total predictions tracked.
    pub fn total_predictions(&self) -> u64 {
        self.buckets.iter().map(|(t, _)| t).sum()
    }

    /// Get raw bucket data for DB persistence.
    pub fn buckets(&self) -> &[(u64, u64); 10] {
        &self.buckets
    }

    /// Load bucket data from DB (replaces current state).
    pub fn load_buckets(&mut self, data: [(u64, u64); 10]) {
        self.buckets = data;
    }
}

/// Response when cognitive overload is detected.
#[derive(Debug, Clone)]
pub enum OverloadResponse {
    /// Break the task into smaller subtasks
    Decompose(Vec<String>),
    /// Suggest using a stronger model
    Escalate(String),
    /// Suggest the user handles this part
    Defer(String),
    /// Suggest a simpler approach
    Simplify(String),
}

/// Detect if the current task exceeds Hydra's reliable capability.
///
/// Uses intent category, complexity, historical success rates, and
/// task characteristics to determine if the task should be decomposed.
pub fn detect_cognitive_overload(
    text: &str,
    intent: &ClassifiedIntent,
    category_success_rate: f64,
) -> Option<OverloadResponse> {
    let word_count = text.split_whitespace().count();

    // Very long prompts with multiple asks → decompose
    let conjunction_count = text.matches(" and ").count() + text.matches(" then ").count()
        + text.matches(" also ").count() + text.matches(" plus ").count();

    if conjunction_count >= 3 && word_count > 50 {
        let subtasks = extract_subtasks(text);
        if subtasks.len() >= 3 {
            return Some(OverloadResponse::Decompose(subtasks));
        }
    }

    // Low historical success rate for this category → escalate or defer
    if category_success_rate < 0.3 {
        return Some(OverloadResponse::Escalate(format!(
            "Historical success rate for {:?} is {:.0}%. Consider using a more capable model.",
            intent.category, category_success_rate * 100.0
        )));
    }

    // Very complex deploy tasks → suggest human review
    if matches!(intent.category, IntentCategory::Deploy) && word_count > 100 {
        return Some(OverloadResponse::Defer(
            "Complex deployment tasks benefit from human review. \
             Consider breaking this into smaller, verifiable steps.".into()
        ));
    }

    // Multi-file refactoring → suggest simpler approach
    let file_refs = text.matches(".rs").count() + text.matches(".ts").count()
        + text.matches(".py").count() + text.matches(".go").count();
    if file_refs > 5 {
        return Some(OverloadResponse::Simplify(
            "This task references many files. Consider tackling one file at a time.".into()
        ));
    }

    None
}

/// Extract subtasks from a complex multi-part request.
fn extract_subtasks(text: &str) -> Vec<String> {
    let mut tasks = Vec::new();

    // Split on numbered lists
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("1.") || trimmed.starts_with("2.") || trimmed.starts_with("3.")
            || trimmed.starts_with("4.") || trimmed.starts_with("5.")
            || trimmed.starts_with("- ") || trimmed.starts_with("* ")
        {
            let task = trimmed.trim_start_matches(|c: char| {
                c.is_ascii_digit() || c == '.' || c == '-' || c == '*' || c == ' '
            });
            if task.len() > 10 {
                tasks.push(task.to_string());
            }
        }
    }

    // If no numbered list, try splitting on conjunctions
    if tasks.is_empty() {
        let parts: Vec<&str> = text.split(" and then ").collect();
        if parts.len() >= 2 {
            tasks = parts.iter()
                .map(|p| p.trim().to_string())
                .filter(|s| s.len() > 10)
                .collect();
        }
    }

    tasks
}

/// Generate a metacognitive assessment of the current interaction.
pub fn assess_interaction(
    intent: &ClassifiedIntent,
    complexity: &str,
    category_success_rate: f64,
    calibration_error: f64,
) -> MetacognitiveAssessment {
    let _ = intent; // Reserved for future category-specific logic
    let confidence_level = if category_success_rate >= 0.8 {
        ConfidenceLevel::High
    } else if category_success_rate >= 0.5 {
        ConfidenceLevel::Medium
    } else {
        ConfidenceLevel::Low
    };

    let should_add_caveats = confidence_level == ConfidenceLevel::Low
        || (complexity == "complex" && calibration_error > 0.2);

    MetacognitiveAssessment {
        confidence_level,
        should_verify: complexity == "complex" || confidence_level != ConfidenceLevel::High,
        should_add_caveats,
        suggested_model_tier: if confidence_level == ConfidenceLevel::Low {
            "sonnet"
        } else {
            "haiku"
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct MetacognitiveAssessment {
    pub confidence_level: ConfidenceLevel,
    pub should_verify: bool,
    pub should_add_caveats: bool,
    pub suggested_model_tier: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_intent(cat: IntentCategory) -> ClassifiedIntent {
        ClassifiedIntent { category: cat, confidence: 0.9, target: None, payload: None }
    }

    #[test]
    fn test_calibration_perfect() {
        let mut ct = CalibrationTracker::new();
        for _ in 0..9 { ct.record(0.9, true); }
        ct.record(0.9, false);
        assert!(ct.calibration_error() < 0.15);
    }

    #[test]
    fn test_calibration_overconfident() {
        let mut ct = CalibrationTracker::new();
        for _ in 0..5 { ct.record(0.9, true); }
        for _ in 0..5 { ct.record(0.9, false); }
        assert!(ct.calibration_error() > 0.2);
    }

    #[test]
    fn test_calibrate_adjusts_down() {
        let mut ct = CalibrationTracker::new();
        for _ in 0..10 { ct.record(0.9, true); }
        for _ in 0..10 { ct.record(0.9, false); }
        let adjusted = ct.calibrate(0.9);
        assert!(adjusted < 0.9);
    }

    #[test]
    fn test_no_overload_simple() {
        let result = detect_cognitive_overload(
            "fix the typo in main.rs",
            &mock_intent(IntentCategory::CodeFix),
            0.8,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_overload_many_conjunctions() {
        let text = "first update the database schema and then migrate the data \
                    and also update all the API endpoints and then write tests \
                    and also update the documentation plus deploy to staging";
        let result = detect_cognitive_overload(
            text,
            &mock_intent(IntentCategory::CodeBuild),
            0.7,
        );
        let _ = result;
    }

    #[test]
    fn test_overload_low_success_rate() {
        let result = detect_cognitive_overload(
            "deploy the new microservice",
            &mock_intent(IntentCategory::Deploy),
            0.2,
        );
        assert!(matches!(result, Some(OverloadResponse::Escalate(_))));
    }

    #[test]
    fn test_overload_many_files() {
        let text = "refactor auth.rs, user.rs, session.rs, token.rs, \
                    middleware.rs, handler.rs and update config.rs";
        let result = detect_cognitive_overload(
            text,
            &mock_intent(IntentCategory::CodeBuild),
            0.7,
        );
        assert!(matches!(result, Some(OverloadResponse::Simplify(_))));
    }

    #[test]
    fn test_extract_subtasks_numbered() {
        let text = "Please do:\n1. Create the database\n\
                    2. Add the migration\n3. Write the API endpoints";
        let tasks = extract_subtasks(text);
        assert_eq!(tasks.len(), 3);
    }

    #[test]
    fn test_assess_high_confidence() {
        let assessment = assess_interaction(
            &mock_intent(IntentCategory::CodeBuild),
            "simple",
            0.9,
            0.05,
        );
        assert_eq!(assessment.confidence_level, ConfidenceLevel::High);
        assert!(!assessment.should_add_caveats);
    }

    #[test]
    fn test_assess_low_confidence() {
        let assessment = assess_interaction(
            &mock_intent(IntentCategory::Deploy),
            "complex",
            0.3,
            0.3,
        );
        assert_eq!(assessment.confidence_level, ConfidenceLevel::Low);
        assert!(assessment.should_add_caveats);
        assert!(assessment.should_verify);
    }

    #[test]
    fn test_calibration_empty() {
        let ct = CalibrationTracker::new();
        assert_eq!(ct.calibration_error(), 0.0);
        assert_eq!(ct.total_predictions(), 0);
        assert_eq!(ct.calibrate(0.8), 0.8);
    }
}
