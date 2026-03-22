//! reporter.rs — Writes HOUR-V2-XX.md and GRAND-REPORT-V2.md

use crate::v2::{
    evaluator::Score,
    analyzer::{Finding, Severity},
};
use std::io::Write;

pub struct V2Run {
    pub hour:   u32,
    pub scores: Vec<Score>,
}

pub fn write_hourly(run: &V2Run) {
    if let Err(e) = std::fs::create_dir_all("harness-reports") {
        eprintln!("Failed to create harness-reports dir: {e}");
        return;
    }
    let path = format!("harness-reports/HOUR-V2-{:02}.md", run.hour);
    let mut f = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create {path}: {e}");
            return;
        }
    };

    let avg = run.scores.iter().map(|s| s.score).sum::<f64>()
            / run.scores.len().max(1) as f64;
    let mem_total = run.scores.iter()
        .filter(|s| s.input_id.starts_with("mem-")).count().max(1);
    let mem_with = run.scores.iter()
        .filter(|s| s.input_id.starts_with("mem-") && s.used_memory).count();
    let memory_rate = mem_with as f64 / mem_total as f64;
    let genome_rate = run.scores.iter()
        .filter(|s| s.used_genome).count() as f64
        / run.scores.len().max(1) as f64;

    let _ = writeln!(f, "# Hydra Harness v2 -- Hour {:02}", run.hour);
    let _ = writeln!(f, "**{}**\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
    let _ = writeln!(f, "## Snapshot");
    let _ = writeln!(f, "| | |");
    let _ = writeln!(f, "|--|--|");
    let _ = writeln!(f, "| Average score | {:.1}/10 |", avg);
    let _ = writeln!(f, "| Memory usage rate | {:.0}% |", memory_rate * 100.0);
    let _ = writeln!(f, "| Genome application rate | {:.0}% |", genome_rate * 100.0);
    let _ = writeln!(f);
    let _ = writeln!(f, "## Scores");
    let _ = writeln!(
        f,
        "| Input | Score | Accurate | Calibrated | Memory | Genome | Finding |"
    );
    let _ = writeln!(
        f,
        "|-------|-------|----------|------------|--------|--------|---------|"
    );

    for s in &run.scores {
        let _ = writeln!(f, "| {} | {:.1} | {} | {} | {} | {} | {} |",
            s.input_id,
            s.score,
            if s.accurate    { "Y" } else { "N" },
            if s.calibrated  { "Y" } else { "N" },
            if s.used_memory { "Y" } else { "-" },
            if s.used_genome { "Y" } else { "-" },
            s.finding,
        );
    }
    println!("  Report written: {path}");
}

pub fn write_grand(all_runs: &[V2Run], findings: &[Finding]) {
    let path = "harness-reports/GRAND-REPORT-V2.md";
    let mut f = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create {path}: {e}");
            return;
        }
    };

    let _ = writeln!(f, "# HYDRA HARNESS V2 -- GRAND REPORT");
    let _ = writeln!(f, "**{}**\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
    let _ = writeln!(f, "**Hours run:** {}", all_runs.len());
    let _ = writeln!(f, "**Total inputs graded:** {}",
        all_runs.iter().map(|r| r.scores.len()).sum::<usize>());

    let _ = writeln!(f, "\n## Score Trend by Hour");
    let _ = writeln!(f, "| Hour | Avg Score | Memory Rate | Genome Rate |");
    let _ = writeln!(f, "|------|-----------|-------------|-------------|");
    for run in all_runs {
        let avg = run.scores.iter().map(|s| s.score).sum::<f64>()
                / run.scores.len().max(1) as f64;
        let mem_t = run.scores.iter()
            .filter(|s| s.input_id.starts_with("mem-")).count().max(1);
        let mem_w = run.scores.iter()
            .filter(|s| s.input_id.starts_with("mem-") && s.used_memory).count();
        let mem = mem_w as f64 / mem_t as f64;
        let gen = run.scores.iter().filter(|s| s.used_genome).count() as f64
                / run.scores.len().max(1) as f64;
        let _ = writeln!(f, "| {:02} | {:.1} | {:.0}% | {:.0}% |",
            run.hour, avg, mem * 100.0, gen * 100.0);
    }

    let _ = writeln!(f, "\n## Findings");
    for finding in findings {
        let icon = match finding.severity {
            Severity::Critical => "CRITICAL",
            Severity::Issue    => "ISSUE",
            Severity::Advisory => "ADVISORY",
            Severity::Healthy  => "HEALTHY",
        };
        let _ = writeln!(f, "\n### [{icon}] {} -- {:?}", finding.name, finding.category);
        let _ = writeln!(f, "**Evidence:** {}", finding.evidence);
        let _ = writeln!(f, "**Cause:** {}", finding.cause);
        let _ = writeln!(f, "**Fix:** {}", finding.fix);
    }

    let fixes: Vec<_> = findings.iter()
        .filter(|fi| matches!(fi.severity, Severity::Issue | Severity::Critical))
        .collect();

    if !fixes.is_empty() {
        let _ = writeln!(f, "\n## What to Make Permanent");
        let _ = writeln!(f,
            "_These are the changes required before concluding Hydra as an entity._\n");
        for (i, finding) in fixes.iter().enumerate() {
            let _ = writeln!(f, "{}. **{}**", i + 1, finding.name);
            let _ = writeln!(f, "   {}\n", finding.fix);
        }
    } else {
        let _ = writeln!(f,
            "\n## Conclusion\n\
             No issues found. All findings are Healthy or Advisory.\n\
             Hydra is operating as intended across all behavioral dimensions.\n\
             The entity is ready to conclude.");
    }

    println!("\n  Grand report: {path}");
}
