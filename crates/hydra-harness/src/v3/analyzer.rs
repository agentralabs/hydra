//! V3 Analyzer — diagnostic findings with evidence → cause → fix triplets.
//! Category-level + cross-category analyses (security posture, learning velocity,
//! timeout analysis, degradation detection).

use super::bank::{V3Category, V3Test};
use super::evaluator::V3HourScores;

#[derive(Debug, Clone)]
pub enum Severity { Healthy, Advisory, Issue, Critical }

impl Severity {
    pub fn label(&self) -> &'static str {
        match self { Self::Healthy => "HEALTHY", Self::Advisory => "ADVISORY",
            Self::Issue => "ISSUE", Self::Critical => "CRITICAL" }
    }
    pub fn icon(&self) -> &'static str {
        match self { Self::Healthy => "✓", Self::Advisory => "○",
            Self::Issue => "▲", Self::Critical => "✗" }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub name: String,
    pub severity: Severity,
    pub evidence: String,
    pub cause: String,
    pub fix: String,
}

/// Analyze all hourly results: category + cross-category diagnostics.
pub fn analyze(all_hours: &[V3HourScores], tests: &[V3Test]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if all_hours.is_empty() { return findings; }

    // Per-category findings
    let categories = V3Category::categories_in(tests);
    for cat in &categories {
        findings.push(analyze_category(*cat, all_hours, tests));
    }

    // Cross-category diagnostics
    findings.extend(analyze_security_posture(all_hours));
    findings.extend(analyze_learning_velocity(all_hours));
    findings.extend(analyze_timeouts(all_hours));
    if all_hours.len() > 1 { findings.extend(analyze_degradation(all_hours)); }

    // Deployment gate
    if all_hours.iter().any(|h| h.deployment_blocked) {
        findings.push(Finding {
            name: "DEPLOYMENT BLOCKED".into(), severity: Severity::Critical,
            evidence: "Blocking category test failure detected".into(),
            cause: "Safety or Security test failed — deployment blocked".into(),
            fix: "Fix ALL blocking test failures before deploying".into(),
        });
    }

    // Recommendation
    let has_critical = findings.iter().any(|f| matches!(f.severity, Severity::Critical));
    let has_issue = findings.iter().any(|f| matches!(f.severity, Severity::Issue));
    if !has_critical && !has_issue {
        findings.push(Finding { name: "RECOMMENDATION".into(), severity: Severity::Healthy,
            evidence: "All systems functional".into(), cause: "".into(),
            fix: "READY FOR 30-DAY USER RUN".into() });
    } else if has_critical {
        findings.push(Finding { name: "RECOMMENDATION".into(), severity: Severity::Critical,
            evidence: "Critical issues detected".into(), cause: "".into(),
            fix: "FIX CRITICAL ISSUES BEFORE DEPLOYMENT".into() });
    } else {
        findings.push(Finding { name: "RECOMMENDATION".into(), severity: Severity::Advisory,
            evidence: "Minor issues detected".into(), cause: "".into(),
            fix: "PROCEED WITH MONITORING".into() });
    }

    findings
}

fn analyze_category(cat: V3Category, all_hours: &[V3HourScores], tests: &[V3Test]) -> Finding {
    let cat_test_ids: Vec<&str> = tests.iter()
        .filter(|t| t.category == cat).map(|t| t.id).collect();
    let total: usize = all_hours.iter()
        .flat_map(|h| h.results.iter().filter(|r| cat_test_ids.contains(&r.test_id.as_str())))
        .count();
    let failures: usize = all_hours.iter()
        .flat_map(|h| h.results.iter().filter(|r| cat_test_ids.contains(&r.test_id.as_str()) && !r.passed))
        .count();
    let last_score = all_hours.last().and_then(|h|
        h.category_scores.iter().find(|(c, _)| *c == cat).map(|(_, s)| *s)
    ).unwrap_or(0.0);
    // Collect per-test details for evidence
    let test_details: Vec<String> = all_hours.last().map(|h| {
        h.capabilities.iter()
            .filter(|c| cat_test_ids.contains(&c.test_id.as_str()))
            .map(|c| format!("{}: {:.0}%", c.test_id, c.percentage))
            .collect()
    }).unwrap_or_default();
    let detail_str = test_details.join(", ");

    if failures == 0 {
        Finding {
            name: cat.label().into(), severity: Severity::Healthy,
            evidence: format!("{total} tests, avg {last_score:.1}/10 [{detail_str}]"),
            cause: "".into(), fix: "".into(),
        }
    } else if cat.is_blocking() {
        let failed: Vec<String> = all_hours.last().map(|h| {
            h.results.iter()
                .filter(|r| cat_test_ids.contains(&r.test_id.as_str()) && !r.passed)
                .map(|r| format!("{}: {}", r.test_id, r.finding)).collect()
        }).unwrap_or_default();
        Finding {
            name: format!("{} FAILURE", cat.label()), severity: Severity::Critical,
            evidence: format!("{failures}/{total} failures [{detail_str}]"),
            cause: format!("Failed: {}", failed.join("; ")),
            fix: "Fix ALL failures immediately — deployment is blocked".into(),
        }
    } else if failures as f64 / total.max(1) as f64 > 0.5 {
        Finding {
            name: format!("{} degraded", cat.label()), severity: Severity::Issue,
            evidence: format!("{failures}/{total} failures, avg {last_score:.1}/10 [{detail_str}]"),
            cause: format!("{} subsystem partially failing", cat.label()),
            fix: "Check individual test results in hourly report".into(),
        }
    } else {
        Finding {
            name: format!("{} partial", cat.label()), severity: Severity::Advisory,
            evidence: format!("{failures}/{total} failures, avg {last_score:.1}/10 [{detail_str}]"),
            cause: "Minor test failures".into(),
            fix: "Monitor in next hour".into(),
        }
    }
}

/// Security posture: injection detection %, redaction, vault permissions.
fn analyze_security_posture(all_hours: &[V3HourScores]) -> Vec<Finding> {
    let last = match all_hours.last() { Some(h) => h, None => return vec![] };
    let sec_tests: Vec<_> = last.capabilities.iter()
        .filter(|c| c.category == "Security" || c.category == "Safety").collect();
    if sec_tests.is_empty() { return vec![]; }
    let avg_pct: f64 = sec_tests.iter().map(|c| c.percentage).sum::<f64>() / sec_tests.len() as f64;
    let low: Vec<_> = sec_tests.iter().filter(|c| c.percentage < 70.0).collect();
    if low.is_empty() {
        vec![Finding {
            name: "Security Posture".into(), severity: Severity::Healthy,
            evidence: format!("Avg {avg_pct:.0}% across {} security tests", sec_tests.len()),
            cause: "".into(), fix: "".into(),
        }]
    } else {
        let details: String = low.iter()
            .map(|c| format!("{}: {:.0}%", c.test_id, c.percentage)).collect::<Vec<_>>().join(", ");
        vec![Finding {
            name: "Security Posture — weak areas".into(), severity: Severity::Advisory,
            evidence: format!("Below 70%: {details}"),
            cause: "Feature extraction vectors need more training data".into(),
            fix: "Run harness over 30 days to accumulate immune antibodies".into(),
        }]
    }
}

/// Learning velocity: genome entry count growing?
fn analyze_learning_velocity(all_hours: &[V3HourScores]) -> Vec<Finding> {
    let genome_counts: Vec<(u32, usize)> = all_hours.iter().filter_map(|h| {
        h.capabilities.iter()
            .find(|c| c.test_id == "learn-1" || c.test_id == "day-learn-1")
            .and_then(|c| {
                c.output_preview.split('=').last()
                    .and_then(|n| n.trim().parse::<usize>().ok())
                    .map(|n| (h.hour, n))
            })
    }).collect();
    if genome_counts.len() < 2 { return vec![]; }
    let first = genome_counts.first().unwrap().1;
    let last = genome_counts.last().unwrap().1;
    if last > first {
        vec![Finding {
            name: "Learning Velocity".into(), severity: Severity::Healthy,
            evidence: format!("Genome grew: {} → {} entries (+{})", first, last, last - first),
            cause: "".into(), fix: "".into(),
        }]
    } else {
        vec![Finding {
            name: "Learning Stalled".into(), severity: Severity::Advisory,
            evidence: format!("Genome static at {} entries across {} hours", last, genome_counts.len()),
            cause: "No new patterns encountered or dream loop not running".into(),
            fix: "Check dream loop configuration and learning sources".into(),
        }]
    }
}

/// Timeout analysis: which tests timeout, and why.
fn analyze_timeouts(all_hours: &[V3HourScores]) -> Vec<Finding> {
    let timeouts: Vec<String> = all_hours.iter().flat_map(|h| {
        h.results.iter()
            .filter(|r| r.finding.contains("TIMEOUT"))
            .map(|r| format!("{} ({}ms, {})", r.test_id, r.duration_ms, r.breakdown.clone()))
    }).collect();
    if timeouts.is_empty() { return vec![]; }
    vec![Finding {
        name: format!("Timeouts: {} tests", timeouts.len()), severity: Severity::Advisory,
        evidence: timeouts.join("; "),
        cause: "Cold boot (5s) + ImmersionMiddleware web search (10s) + LLM API latency".into(),
        fix: "Pre-warm genome with common patterns OR increase timeout for LLM tests".into(),
    }]
}

/// Degradation detection: tests that passed in hour N but fail in hour N+1.
fn analyze_degradation(all_hours: &[V3HourScores]) -> Vec<Finding> {
    let mut regressions = Vec::new();
    for i in 1..all_hours.len() {
        let prev = &all_hours[i - 1];
        let curr = &all_hours[i];
        for r in &curr.results {
            if !r.passed {
                if let Some(prev_r) = prev.results.iter().find(|p| p.test_id == r.test_id) {
                    if prev_r.passed {
                        regressions.push(format!("{} (hour {} → {})", r.test_id, i, i + 1));
                    }
                }
            }
        }
    }
    if regressions.is_empty() { return vec![]; }
    vec![Finding {
        name: "Regressions Detected".into(), severity: Severity::Issue,
        evidence: regressions.join(", "),
        cause: "Tests that previously passed are now failing".into(),
        fix: "Check if system state changed between hours (genome corruption, lock stale)".into(),
    }]
}
