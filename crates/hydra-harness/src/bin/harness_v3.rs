//! harness_v3.rs — Combined V3 Test Loop.
//! Runs Part A (32 ops) + Part B (28 day) + Part C (28 orch) = 88 tests.
//! Usage: cargo run -p hydra-harness --bin harness_v3 -- --hours 5 [--suite ops|day|orch|all]

use hydra_harness::v3::{
    bank::{self, V3Category},
    runner::{run_test, fix_vault_permissions},
    evaluator::score_hour,
    analyzer::analyze,
    reporter::{write_hourly, write_grand},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total_hours: u32 = args.iter()
        .position(|a| a == "--hours")
        .and_then(|i| args.get(i + 1))
        .and_then(|h| h.parse().ok())
        .unwrap_or(5);

    let suite = args.iter()
        .position(|a| a == "--suite")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("all");

    fix_vault_permissions();

    // Pre-build Hydra binary so subprocess tests have zero compiler output
    println!("  Pre-building Hydra binary...");
    let _ = std::process::Command::new("cargo")
        .args(["build", "-q", "-p", "hydra-kernel", "--bin", "hydra"])
        .status();
    println!("  Build complete.\n");

    let tests = match suite {
        "ops" => bank::ops_bank(),
        "day" => bank::day_bank(),
        "orch" => bank::orch_bank(),
        _ => bank::test_bank(),
    };

    let _categories = V3Category::categories_in(&tests);
    let suite_label = match suite {
        "ops" => "OPERATIONAL CALIBRATION (32 tests, 6 categories)",
        "day" => "REAL USER DAY (28 tests, 7 categories)",
        "orch" => "ORCHESTRATION COVERAGE (28 tests, 4 categories)",
        _ => "COMBINED (88 tests, 17 categories)",
    };

    println!("===========================================================");
    println!("  HYDRA HARNESS V3 — {suite_label}");
    println!("  {} hours | {} tests per hour", total_hours, tests.len());
    println!("  Blocking: Safety + Security (ANY failure = BLOCKED)");
    println!("===========================================================\n");

    let mut all_hours = Vec::new();

    for hour in 1..=total_hours {
        println!("  ── Hour {hour}/{total_hours} ──\n");
        let mut results = Vec::new();
        let mut current_cat = None;

        for test in &tests {
            if current_cat != Some(test.category) {
                current_cat = Some(test.category);
                let blocking = if test.category.is_blocking() { " [BLOCKING]" } else { "" };
                println!("    ─ {}{} ─", test.category.label(), blocking);
            }
            print!("    {:<40}", test.name);
            let result = run_test(test, hour);
            let icon = if result.passed { "✓" } else { "✗" };
            // Show % instead of /10
            println!(" {icon} {:>3.0}%  {:>5}ms  {}",
                result.percentage, result.duration_ms, result.finding);
            if !result.passed {
                // Print failure detail immediately
                println!("      └─ {}", result.breakdown);
            }
            results.push(result);
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        let scores = score_hour(hour, results, &tests);

        // Category summary table
        println!();
        println!("  ┌────────────────────┬─────────┬────────┬─────────────────────────────────────────┐");
        println!("  │ Category           │  Score  │  Pass  │ Per-Test Scores                         │");
        println!("  ├────────────────────┼─────────┼────────┼─────────────────────────────────────────┤");
        for (cat, score) in &scores.category_scores {
            let flag = if cat.is_blocking() { "*" } else { " " };
            let cat_caps: Vec<_> = scores.capabilities.iter()
                .filter(|c| c.category == cat.label()).collect();
            let pass_count = cat_caps.iter().filter(|c| c.passed).count();
            let pcts: String = cat_caps.iter()
                .map(|c| format!("{:.0}%", c.percentage))
                .collect::<Vec<_>>().join(" ");
            let pcts = if pcts.len() > 39 { format!("{}...", &pcts[..36]) } else { pcts };
            println!("  │{}{:<19}│ {:>4.1}/10 │ {}/{:<3} │ {:<39} │",
                flag, cat.label(), score, pass_count, cat_caps.len(), pcts);
        }
        println!("  ├────────────────────┼─────────┼────────┼─────────────────────────────────────────┤");
        let status = if scores.deployment_blocked { "BLOCKED" } else { "CLEAR" };
        println!("  │ OVERALL            │ {:>4.1}/10 │ {}/{:<3} │ deployment: {:<27} │",
            scores.overall, scores.tests_passed, scores.tests_total, status);
        if scores.total_tokens > 0 {
            println!("  │                    │         │        │ tokens: {:<31} │", scores.total_tokens);
        }
        println!("  └────────────────────┴─────────┴────────┴─────────────────────────────────────────┘");
        if scores.deployment_blocked { println!("  * = BLOCKING category"); }

        // Top failures
        let mut failures: Vec<_> = scores.capabilities.iter()
            .filter(|c| !c.passed).collect();
        failures.sort_by(|a, b| a.percentage.partial_cmp(&b.percentage).unwrap());
        if !failures.is_empty() {
            println!("\n  Failures:");
            for f in &failures {
                println!("    ✗ {} ({:.0}%) — {}", f.test_id, f.percentage, f.breakdown);
            }
        }

        // Lowest scoring capabilities (even passing)
        let mut lowest: Vec<_> = scores.capabilities.iter()
            .filter(|c| c.percentage < 100.0 && c.passed).collect();
        lowest.sort_by(|a, b| a.percentage.partial_cmp(&b.percentage).unwrap());
        if !lowest.is_empty() {
            println!("\n  Attention (below 100%):");
            for c in lowest.iter().take(5) {
                println!("    ○ {} ({:.0}%) — {}", c.test_id, c.percentage, c.name);
            }
        }
        println!();

        write_hourly(&scores);
        all_hours.push(scores);

        if hour < total_hours {
            println!("  Waiting 1 hour for next cycle...\n");
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }

    println!("\n  Analyzing {} hours of data...", all_hours.len());
    let findings = analyze(&all_hours, &tests);

    println!("\n  Findings:");
    for f in &findings {
        println!("    {} {}: {}", f.severity.icon(), f.severity.label(), f.name);
        if !f.evidence.is_empty() { println!("       Evidence: {}", f.evidence); }
        if !f.cause.is_empty() { println!("       Cause: {}", f.cause); }
        if !f.fix.is_empty() { println!("       Fix: {}", f.fix); }
    }

    write_grand(&all_hours, &findings);

    let blocked = all_hours.last().map(|h| h.deployment_blocked).unwrap_or(false);
    println!("\n===========================================================");
    println!("  V3 HARNESS COMPLETE — {suite_label}");
    if blocked {
        println!("  DEPLOYMENT: BLOCKED");
    } else {
        println!("  DEPLOYMENT: CLEAR");
    }
    println!("  Review: harness-reports/GRAND-REPORT-V3.md");
    println!("===========================================================");
}
