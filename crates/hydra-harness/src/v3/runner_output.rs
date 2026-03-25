//! V3 OutputCheck runner — verifies rich output classification.
//! Tests that Hydra correctly classifies tables, diffs, code blocks, etc.

use super::bank::V3Test;
use super::runner::V3Result;

/// Run an OutputCheck test by classifying the input through RichOutput.
pub fn run_output_check(test: &V3Test) -> V3Result {
    let classified = hydra_kernel::rich_output::classify_output(test.input);
    let type_label = classified.type_label();

    let passed = if test.pass_contains.is_empty() {
        type_label != "text" || test.input.len() < 20
    } else {
        test.pass_contains.iter().any(|expected| type_label == *expected)
    };

    let score = if passed { 10.0 } else { 0.0 };
    let pct = if passed { 100.0 } else { 0.0 };

    V3Result {
        test_id: test.id.to_string(), passed, score,
        output: format!("classified_as={type_label}"), duration_ms: 0,
        finding: if passed {
            format!("Correctly classified as '{type_label}'")
        } else {
            format!("FAIL: expected {:?}, got '{type_label}'", test.pass_contains)
        },
        receipt: None, percentage: pct,
        breakdown: format!("output_check={pct:.0}% (classified={type_label})"),
    }
}
