//! V3 Reporter — generates hourly and grand reports with:
//! - Capability scorecard (every test with %)
//! - Failure deep-dive (full output + probable cause)
//! - Cross-hour trend analysis
//! - "What to Make Permanent" actionable fix list
//! - Token economics

use super::evaluator::V3HourScores;
use super::analyzer::{Finding, Severity};

/// Write hourly report with capability scorecard.
pub fn write_hourly(scores: &V3HourScores) {
    let dir = "harness-reports";
    let _ = std::fs::create_dir_all(dir);
    let path = format!("{dir}/HOUR-V3-{:02}.md", scores.hour);

    let mut r = format!("# V3 Test — Hour {}\n\n", scores.hour);
    if scores.deployment_blocked {
        r.push_str("## !! DEPLOYMENT BLOCKED !!\n\n");
    }

    // Category summary
    r.push_str("## Category Scores\n\n");
    r.push_str("| Category | Score | Status |\n|---|---|---|\n");
    for (cat, score) in &scores.category_scores {
        let status = if *score >= 9.0 { "✓" } else if *score >= 5.0 { "○" } else { "✗" };
        let block = if cat.is_blocking() { " **[BLOCK]**" } else { "" };
        r.push_str(&format!("| {}{} | {:.1}/10 | {} |\n", cat.label(), block, score, status));
    }
    r.push_str(&format!("| **OVERALL** | **{:.1}/10** | {} |\n\n",
        scores.overall, if scores.deployment_blocked { "BLOCKED" } else { "OK" }));
    r.push_str(&format!("Pass: {}/{}  |  Tokens: {}  |  Duration: {}ms\n\n",
        scores.tests_passed, scores.tests_total, scores.total_tokens, scores.total_duration_ms));

    // Capability scorecard — every test with %
    r.push_str("## Capability Scorecard\n\n");
    r.push_str("| % | ID | Capability | Category | Time | Tokens | Finding |\n");
    r.push_str("|---|---|---|---|---|---|---|\n");
    for c in &scores.capabilities {
        let icon = if c.passed { "✓" } else { "✗" };
        let tok = if c.tokens > 0 { format!("{}", c.tokens) } else { "-".into() };
        let dur = if c.duration_ms > 0 { format!("{}ms", c.duration_ms) } else { "-".into() };
        let finding_short = &c.output_preview[..c.output_preview.len().min(80)];
        r.push_str(&format!("| {icon} {:.0}% | {} | {} | {} | {} | {} | {} |\n",
            c.percentage, c.test_id, c.name, c.category, dur, tok, finding_short));
    }
    r.push('\n');

    // Failure deep-dive
    let failures: Vec<_> = scores.capabilities.iter().filter(|c| !c.passed).collect();
    if !failures.is_empty() {
        r.push_str("## Failure Deep-Dive\n\n");
        for f in &failures {
            r.push_str(&format!("### {} {} — {:.0}%\n", f.test_id, f.name, f.percentage));
            r.push_str(&format!("- **Category**: {}\n", f.category));
            r.push_str(&format!("- **Breakdown**: {}\n", f.breakdown));
            r.push_str(&format!("- **Duration**: {}ms\n", f.duration_ms));
            if f.tokens > 0 { r.push_str(&format!("- **Tokens**: {}\n", f.tokens)); }
            r.push_str(&format!("- **Output**: {}\n\n", f.output_preview));
        }
    }

    if let Err(e) = std::fs::write(&path, &r) {
        eprintln!("V3: failed to write {path}: {e}");
    } else {
        println!("  Report: {path}");
    }
}

/// Write grand report with trends, findings, failure analysis, and action items.
pub fn write_grand(all_hours: &[V3HourScores], findings: &[Finding]) {
    let dir = "harness-reports";
    let _ = std::fs::create_dir_all(dir);
    let path = format!("{dir}/GRAND-REPORT-V3.md");

    let mut r = format!("# V3 Grand Report — {} Hours\n\n", all_hours.len());

    let blocked = all_hours.last().map(|h| h.deployment_blocked).unwrap_or(false);
    r.push_str(&format!("## DEPLOYMENT STATUS: **{}**\n\n",
        if blocked { "BLOCKED" } else { "CLEAR" }));

    // Token economics
    let total_tok: usize = all_hours.iter().map(|h| h.total_tokens).sum();
    let total_dur: u64 = all_hours.iter().map(|h| h.total_duration_ms).sum();
    if total_tok > 0 || total_dur > 0 {
        r.push_str("## Token Economics\n\n");
        r.push_str(&format!("- Total tokens: {total_tok}\n"));
        r.push_str(&format!("- Total test duration: {:.1}s\n", total_dur as f64 / 1000.0));
        let tests_total: usize = all_hours.iter().map(|h| h.tests_total).sum();
        if tests_total > 0 {
            r.push_str(&format!("- Avg tokens/test: {}\n", total_tok / tests_total.max(1)));
        }
        r.push('\n');
    }

    // Hourly trend
    r.push_str("## Hourly Trend\n\n");
    if let Some(first) = all_hours.first() {
        r.push_str("| Hour | Overall |");
        for (cat, _) in &first.category_scores { r.push_str(&format!(" {} |", cat.label())); }
        r.push_str(" Pass | Blocked |\n|---|---|");
        for _ in &first.category_scores { r.push_str("---|"); }
        r.push_str("---|---|\n");
        for h in all_hours {
            r.push_str(&format!("| {} | {:.1} |", h.hour, h.overall));
            for (_, score) in &h.category_scores { r.push_str(&format!(" {:.1} |", score)); }
            r.push_str(&format!(" {}/{} | {} |\n",
                h.tests_passed, h.tests_total,
                if h.deployment_blocked { "YES" } else { "no" }));
        }
        r.push('\n');
    }

    // Final score
    if let Some(last) = all_hours.last() {
        r.push_str(&format!("## Final Score: {:.1}/10\n\n", last.overall));
        r.push_str(&format!("Tests passing: {}/{} ({:.0}%)\n\n",
            last.tests_passed, last.tests_total,
            last.tests_passed as f64 / last.tests_total.max(1) as f64 * 100.0));
    }

    // Capability scorecard (last hour)
    if let Some(last) = all_hours.last() {
        r.push_str("## Capability Scorecard (Latest)\n\n");
        r.push_str("| % | ID | Capability | Category | Finding |\n|---|---|---|---|---|\n");
        for c in &last.capabilities {
            let icon = if c.passed { "✓" } else { "✗" };
            let preview = &c.output_preview[..c.output_preview.len().min(60)];
            r.push_str(&format!("| {icon} {:.0}% | {} | {} | {} | {} |\n",
                c.percentage, c.test_id, c.name, c.category, preview));
        }
        r.push('\n');
    }

    // Findings
    r.push_str("## Findings\n\n");
    for f in findings {
        r.push_str(&format!("### {} {}: {}\n", f.severity.icon(), f.severity.label(), f.name));
        if !f.evidence.is_empty() { r.push_str(&format!("- Evidence: {}\n", f.evidence)); }
        if !f.cause.is_empty() { r.push_str(&format!("- Cause: {}\n", f.cause)); }
        if !f.fix.is_empty() { r.push_str(&format!("- Fix: {}\n", f.fix)); }
        r.push('\n');
    }

    // Failure deep-dive (all hours)
    let mut all_bugs: Vec<String> = Vec::new();
    for h in all_hours {
        for c in &h.capabilities {
            if !c.passed {
                let bug = format!("**[Hour {}] {} — {} ({:.0}%)**\n  Breakdown: {}\n  Output: {}",
                    h.hour, c.test_id, c.name, c.percentage, c.breakdown,
                    &c.output_preview[..c.output_preview.len().min(200)]);
                if !all_bugs.iter().any(|b| b.contains(&c.test_id)) { all_bugs.push(bug); }
            }
        }
    }
    r.push_str("## Bugs Detected\n\n");
    if all_bugs.is_empty() {
        r.push_str("No bugs detected.\n\n");
    } else {
        for b in &all_bugs { r.push_str(&format!("{b}\n\n")); }
    }

    // What to Make Permanent
    let fixes: Vec<_> = findings.iter()
        .filter(|f| matches!(f.severity, Severity::Issue | Severity::Critical) && !f.fix.is_empty())
        .collect();
    if !fixes.is_empty() {
        r.push_str("## What to Make Permanent\n\n");
        for (i, f) in fixes.iter().enumerate() {
            r.push_str(&format!("{}. **{}**: {}\n", i + 1, f.name, f.fix));
        }
        r.push('\n');
    } else {
        r.push_str("## Conclusion\n\n");
        r.push_str("No critical or issue-level findings. All capabilities operational.\n");
        r.push_str("Hydra is ready for 30-day user run.\n\n");
    }

    if let Err(e) = std::fs::write(&path, &r) {
        eprintln!("V3: failed to write {path}: {e}");
    } else {
        println!("\n  Grand Report: {path}");
    }
}
