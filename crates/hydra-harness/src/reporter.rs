//! reporter.rs — Writes hourly MD reports and grand report.

use crate::runner::HarnessRun;
use std::io::Write;
use std::path::PathBuf;

/// Write an hourly report for a single harness run.
pub fn write_hourly_report(run: &HarnessRun) {
    let _ = std::fs::create_dir_all("harness-reports");
    let path = PathBuf::from(format!("harness-reports/HOUR-{:02}.md", run.hour));
    let file = std::fs::File::create(&path);
    let mut file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  Failed to create report {}: {}", path.display(), e);
            return;
        }
    };

    let _ = writeln!(file, "# Hydra Harness -- Hour {:02}", run.hour);
    let _ = writeln!(file, "**Started:** {}",
        run.started.format("%Y-%m-%d %H:%M:%S UTC"));
    let _ = writeln!(file, "**Ended:** {}",
        run.ended.format("%Y-%m-%d %H:%M:%S UTC"));
    let _ = writeln!(file, "**Duration:** {}s",
        (run.ended - run.started).num_seconds());
    let _ = writeln!(file);
    let _ = writeln!(file, "## Summary");
    let _ = writeln!(file, "| Metric | Value |");
    let _ = writeln!(file, "|--------|-------|");
    let _ = writeln!(file, "| Total tests | {} |", run.total());
    let _ = writeln!(file, "| Passed | {} |", run.passed());
    let _ = writeln!(file, "| Failed | {} |", run.failed());
    let _ = writeln!(file, "| Auto-fixed | {} |", run.fixed());

    if run.total() > 0 {
        let _ = writeln!(file, "| Pass rate | {:.1}% |",
            run.passed() as f64 / run.total() as f64 * 100.0);
    }
    let _ = writeln!(file);

    let _ = writeln!(file, "## Results by Crate");
    let _ = writeln!(file, "| Crate | Capability | Result | Duration | Notes |");
    let _ = writeln!(file, "|-------|-----------|--------|----------|-------|");

    for r in &run.results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        let fix_note = match (r.fix_attempted, &r.fix_succeeded) {
            (true, Some(true))  => " -> FIXED",
            (true, Some(false)) => " -> FIX FAILED",
            (true, None)        => " -> FIX IN PROGRESS",
            _                   => "",
        };
        let error_note = r.error.as_deref().unwrap_or("--");
        let _ = writeln!(file,
            "| {} | {} | {}{} | {}ms | {} |",
            r.crate_name, r.capability, status, fix_note,
            r.duration_ms, error_note,
        );
    }

    write_failures_section(&mut file, run);

    println!("\n  Report written: {}", path.display());
}

fn write_failures_section(file: &mut std::fs::File, run: &HarnessRun) {
    if run.failed() == 0 {
        return;
    }
    let _ = writeln!(file);
    let _ = writeln!(file, "## Failures Requiring Attention");
    for r in run.results.iter().filter(|r| !r.passed) {
        let _ = writeln!(file);
        let _ = writeln!(file, "### {} -- {}", r.crate_name, r.capability);
        let _ = writeln!(file, "**Error:** {}",
            r.error.as_deref().unwrap_or("unknown"));
        if let Some(notes) = &r.fix_notes {
            let _ = writeln!(file, "**Fix attempt:** {}", notes);
        }
        if r.fix_succeeded == Some(false) {
            let _ = writeln!(file, "**Needs manual review**");
        }
    }
}

/// Write the grand report aggregating all hourly runs.
pub fn write_grand_report(all_runs: &[HarnessRun]) {
    let path = PathBuf::from("harness-reports/GRAND-REPORT.md");
    let file = std::fs::File::create(&path);
    let mut file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  Failed to create grand report: {}", e);
            return;
        }
    };

    let total_tests: usize = all_runs.iter().map(|r| r.total()).sum();
    let total_passed: usize = all_runs.iter().map(|r| r.passed()).sum();
    let total_failed: usize = all_runs.iter().map(|r| r.failed()).sum();
    let total_fixed: usize = all_runs.iter().map(|r| r.fixed()).sum();

    let _ = writeln!(file, "# HYDRA HARNESS -- GRAND REPORT");
    let _ = writeln!(file, "**Duration:** {} hours", all_runs.len());
    let _ = writeln!(file, "**Generated:** {}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
    let _ = writeln!(file);
    let _ = writeln!(file, "## Overall Statistics");
    let _ = writeln!(file, "| Metric | Value |");
    let _ = writeln!(file, "|--------|-------|");
    let _ = writeln!(file, "| Hours run | {} |", all_runs.len());
    let _ = writeln!(file, "| Total test executions | {} |", total_tests);
    let _ = writeln!(file, "| Total passed | {} |", total_passed);
    let _ = writeln!(file, "| Total failed | {} |", total_failed);
    let _ = writeln!(file, "| Auto-fixed | {} |", total_fixed);

    if total_tests > 0 {
        let _ = writeln!(file, "| Overall pass rate | {:.1}% |",
            total_passed as f64 / total_tests as f64 * 100.0);
    }
    let _ = writeln!(file);

    write_persistent_failures(&mut file, all_runs);
    write_auto_fixed(&mut file, all_runs);
    write_recommendations(&mut file);

    println!("\n  Grand report written: {}", path.display());
}

fn write_persistent_failures(file: &mut std::fs::File, all_runs: &[HarnessRun]) {
    let _ = writeln!(file, "## Persistent Failures (failed > 50% of hours)");
    let _ = writeln!(file, "_These need permanent fixes._");
    let _ = writeln!(file);

    let mut counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for run in all_runs {
        for r in run.results.iter().filter(|r| !r.passed) {
            let key = format!("{}::{}", r.crate_name, r.capability);
            *counts.entry(key).or_default() += 1;
        }
    }

    let threshold = all_runs.len() / 2;
    let mut persistent: Vec<_> = counts.iter()
        .filter(|(_, &count)| count > threshold)
        .collect();
    persistent.sort_by(|a, b| b.1.cmp(a.1));

    if persistent.is_empty() {
        let _ = writeln!(file,
            "_None -- all failures were transient or auto-fixed._");
    } else {
        for (key, count) in &persistent {
            let _ = writeln!(file, "- **{}** -- failed {}/{} hours",
                key, count, all_runs.len());
        }
    }
}

fn write_auto_fixed(file: &mut std::fs::File, all_runs: &[HarnessRun]) {
    let _ = writeln!(file);
    let _ = writeln!(file, "## Auto-Fixed Issues");
    let _ = writeln!(file,
        "_These were fixed automatically and need permanent hardening._");
    let _ = writeln!(file);

    for run in all_runs {
        for r in run.results.iter().filter(|r| r.fix_succeeded == Some(true)) {
            let _ = writeln!(file, "- Hour {:02}: **{}::{}** -- {}",
                run.hour, r.crate_name, r.capability,
                r.fix_notes.as_deref().unwrap_or("auto-fixed"));
        }
    }
}

fn write_recommendations(file: &mut std::fs::File) {
    let _ = writeln!(file);
    let _ = writeln!(file, "## Recommendations for Permanent Hardening");
    let _ = writeln!(file);
    let _ = writeln!(file,
        "1. Investigate root cause for all persistent failures above");
    let _ = writeln!(file,
        "2. Move auto-fixes into the crate source permanently");
    let _ = writeln!(file,
        "3. Add regression tests for any crate with > 0 failures");
}
