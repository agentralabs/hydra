//! ABTester — A/B testing for pattern mutations.

use serde::{Deserialize, Serialize};

/// A variant in an A/B test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub name: String,
    pub actions: Vec<String>,
    pub successes: u64,
    pub failures: u64,
    pub total_duration_ms: f64,
}

impl Variant {
    pub fn new(name: &str, actions: Vec<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            actions,
            successes: 0,
            failures: 0,
            total_duration_ms: 0.0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 0.0;
        }
        self.successes as f64 / total as f64
    }

    pub fn avg_duration(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 0.0;
        }
        self.total_duration_ms / total as f64
    }

    pub fn total_executions(&self) -> u64 {
        self.successes + self.failures
    }
}

/// An A/B test comparing two variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    pub id: String,
    pub name: String,
    pub variant_a: Variant,
    pub variant_b: Variant,
    pub min_samples: u64,
    pub status: TestStatus,
    pub winner: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Running,
    Concluded,
    Insufficient,
}

/// Manages A/B tests for pattern mutations
pub struct ABTester {
    tests: parking_lot::RwLock<Vec<ABTest>>,
    confidence_threshold: f64,
}

impl ABTester {
    pub fn new(confidence_threshold: f64) -> Self {
        Self {
            tests: parking_lot::RwLock::new(Vec::new()),
            confidence_threshold: confidence_threshold.clamp(0.5, 0.99),
        }
    }

    /// Create a new A/B test
    pub fn create_test(
        &self,
        name: &str,
        variant_a: Variant,
        variant_b: Variant,
        min_samples: u64,
    ) -> String {
        let test = ABTest {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            variant_a,
            variant_b,
            min_samples,
            status: TestStatus::Running,
            winner: None,
        };
        let id = test.id.clone();
        self.tests.write().push(test);
        id
    }

    /// Record a result for a variant
    pub fn record_result(
        &self,
        test_id: &str,
        variant_name: &str,
        success: bool,
        duration_ms: f64,
    ) -> bool {
        let mut tests = self.tests.write();
        if let Some(test) = tests.iter_mut().find(|t| t.id == test_id) {
            let variant = if test.variant_a.name == variant_name {
                &mut test.variant_a
            } else if test.variant_b.name == variant_name {
                &mut test.variant_b
            } else {
                return false;
            };

            if success {
                variant.successes += 1;
            } else {
                variant.failures += 1;
            }
            variant.total_duration_ms += duration_ms;

            // Check if we can conclude
            self.check_conclusion(test);
            true
        } else {
            false
        }
    }

    fn check_conclusion(&self, test: &mut ABTest) {
        let a_total = test.variant_a.total_executions();
        let b_total = test.variant_b.total_executions();

        if a_total < test.min_samples || b_total < test.min_samples {
            test.status = TestStatus::Running;
            return;
        }

        let a_rate = test.variant_a.success_rate();
        let b_rate = test.variant_b.success_rate();
        let diff = (a_rate - b_rate).abs();

        // Simple significance check: difference > (1 - threshold)
        if diff > (1.0 - self.confidence_threshold) {
            test.status = TestStatus::Concluded;
            test.winner = if a_rate > b_rate {
                Some(test.variant_a.name.clone())
            } else {
                Some(test.variant_b.name.clone())
            };
        }
    }

    /// Get test by ID
    pub fn get_test(&self, test_id: &str) -> Option<ABTest> {
        self.tests.read().iter().find(|t| t.id == test_id).cloned()
    }

    /// Get winner of a concluded test
    pub fn winner(&self, test_id: &str) -> Option<String> {
        self.tests
            .read()
            .iter()
            .find(|t| t.id == test_id)
            .and_then(|t| t.winner.clone())
    }

    /// List all tests
    pub fn list(&self) -> Vec<ABTest> {
        self.tests.read().clone()
    }

    pub fn test_count(&self) -> usize {
        self.tests.read().len()
    }
}

impl Default for ABTester {
    fn default() -> Self {
        Self::new(0.95)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ab_test_setup() {
        let tester = ABTester::new(0.9);
        let a = Variant::new("original", vec!["read".into(), "write".into()]);
        let b = Variant::new(
            "mutated",
            vec!["read".into(), "validate".into(), "write".into()],
        );

        let test_id = tester.create_test("edit_flow_test", a, b, 10);
        let test = tester.get_test(&test_id).unwrap();
        assert_eq!(test.status, TestStatus::Running);
        assert!(test.winner.is_none());
    }

    #[test]
    fn test_ab_test_winner() {
        let tester = ABTester::new(0.8);
        let a = Variant::new("variant_a", vec!["step1".into()]);
        let b = Variant::new("variant_b", vec!["step2".into()]);

        let test_id = tester.create_test("test", a, b, 5);

        // Variant A: 80% success
        for _ in 0..8 {
            tester.record_result(&test_id, "variant_a", true, 100.0);
        }
        for _ in 0..2 {
            tester.record_result(&test_id, "variant_a", false, 100.0);
        }

        // Variant B: 30% success
        for _ in 0..3 {
            tester.record_result(&test_id, "variant_b", true, 100.0);
        }
        for _ in 0..7 {
            tester.record_result(&test_id, "variant_b", false, 100.0);
        }

        let test = tester.get_test(&test_id).unwrap();
        assert_eq!(test.status, TestStatus::Concluded);
        assert_eq!(test.winner, Some("variant_a".into()));
    }
}
