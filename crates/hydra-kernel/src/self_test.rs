//! Periodic behavioral self-test — runs known test questions in the dream loop.
//! Tracks score trends over time. Alerts if scores drop.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A self-test question with expected characteristics.
#[derive(Debug, Clone)]
pub struct TestQuestion {
    pub input: &'static str,
    pub expected_contains: &'static [&'static str],
    pub category: &'static str,
}

/// Result of one self-test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfTestResult {
    pub total: usize,
    pub passed: usize,
    pub score: f64,
    pub timestamp: DateTime<Utc>,
    pub failures: Vec<String>,
}

/// Known test questions for behavioral validation.
pub const TEST_QUESTIONS: &[TestQuestion] = &[
    TestQuestion {
        input: "What is 2 + 2?",
        expected_contains: &["4"],
        category: "basic-arithmetic",
    },
    TestQuestion {
        input: "What is a circuit breaker pattern?",
        expected_contains: &["fail", "service", "prevent"],
        category: "software-knowledge",
    },
    TestQuestion {
        input: "Explain the CAP theorem briefly",
        expected_contains: &["consistency", "availability", "partition"],
        category: "distributed-systems",
    },
];

/// Tracker for self-test scores over time.
pub struct SelfTestTracker {
    history: Vec<SelfTestResult>,
    max_history: usize,
}

impl SelfTestTracker {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 100,
        }
    }

    /// Record a test result.
    pub fn record(&mut self, result: SelfTestResult) {
        if result.score < self.last_score().unwrap_or(1.0) - 0.2 {
            eprintln!(
                "hydra: SELF-TEST SCORE DROP — {:.0}% → {:.0}%",
                self.last_score().unwrap_or(0.0) * 100.0,
                result.score * 100.0
            );
        }
        self.history.push(result);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Evaluate a response against expected keywords.
    pub fn evaluate_response(question: &TestQuestion, response: &str) -> bool {
        let lower = response.to_lowercase();
        question
            .expected_contains
            .iter()
            .all(|keyword| lower.contains(&keyword.to_lowercase()))
    }

    /// Run all test questions against a response generator.
    /// Returns a SelfTestResult without actually calling the LLM.
    /// (The dream loop calls this with responses it has.)
    pub fn evaluate_batch(responses: &[(&TestQuestion, &str)]) -> SelfTestResult {
        let total = responses.len();
        let mut passed = 0;
        let mut failures = Vec::new();

        for (question, response) in responses {
            if Self::evaluate_response(question, response) {
                passed += 1;
            } else {
                failures.push(format!(
                    "{}: expected {:?}",
                    question.category, question.expected_contains
                ));
            }
        }

        let score = if total > 0 {
            passed as f64 / total as f64
        } else {
            1.0
        };

        SelfTestResult {
            total,
            passed,
            score,
            timestamp: Utc::now(),
            failures,
        }
    }

    pub fn last_score(&self) -> Option<f64> {
        self.history.last().map(|r| r.score)
    }

    pub fn trend(&self) -> &str {
        if self.history.len() < 2 {
            return "insufficient data";
        }
        let recent = self.history.last().unwrap().score;
        let prev = self.history[self.history.len() - 2].score;
        if recent > prev + 0.05 {
            "improving"
        } else if recent < prev - 0.05 {
            "declining"
        } else {
            "stable"
        }
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

impl Default for SelfTestTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_matching_response() {
        let q = &TEST_QUESTIONS[0]; // "What is 2 + 2?"
        assert!(SelfTestTracker::evaluate_response(q, "The answer is 4."));
    }

    #[test]
    fn evaluate_missing_keywords() {
        let q = &TEST_QUESTIONS[0];
        assert!(!SelfTestTracker::evaluate_response(q, "I don't know."));
    }

    #[test]
    fn batch_evaluation() {
        let q = &TEST_QUESTIONS[0];
        let result = SelfTestTracker::evaluate_batch(&[(q, "The answer is 4")]);
        assert_eq!(result.passed, 1);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn score_drop_detection() {
        let mut tracker = SelfTestTracker::new();
        tracker.record(SelfTestResult {
            total: 3,
            passed: 3,
            score: 1.0,
            timestamp: Utc::now(),
            failures: vec![],
        });
        tracker.record(SelfTestResult {
            total: 3,
            passed: 1,
            score: 0.33,
            timestamp: Utc::now(),
            failures: vec!["test".into()],
        });
        assert_eq!(tracker.trend(), "declining");
    }

    #[test]
    fn stable_trend() {
        let mut tracker = SelfTestTracker::new();
        for _ in 0..3 {
            tracker.record(SelfTestResult {
                total: 3,
                passed: 3,
                score: 1.0,
                timestamp: Utc::now(),
                failures: vec![],
            });
        }
        assert_eq!(tracker.trend(), "stable");
    }
}
