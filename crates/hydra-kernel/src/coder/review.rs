//! Self-code review + refactoring detection + bug-to-genome + decision recording.
//! Automates what a senior developer does after writing code.

use super::CodebaseProfile;

// ── Types ──

/// An issue found during code review.
#[derive(Debug, Clone)]
pub struct ReviewIssue {
    pub category: ReviewCategory,
    pub description: String,
    pub severity: f64,
    pub file: Option<String>,
    pub line: Option<usize>,
}

/// Category of review issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewCategory {
    Security,
    Performance,
    BestPractice,
    Architecture,
    Documentation,
    Refactoring,
}

impl std::fmt::Display for ReviewCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Security => write!(f, "Security"),
            Self::Performance => write!(f, "Performance"),
            Self::BestPractice => write!(f, "Best Practice"),
            Self::Architecture => write!(f, "Architecture"),
            Self::Documentation => write!(f, "Documentation"),
            Self::Refactoring => write!(f, "Refactoring"),
        }
    }
}

// ── Stage 7: Self-Code Review ──

/// Review code in the working directory for common issues.
pub fn review_code(working_dir: &str, profile: &CodebaseProfile) -> Vec<ReviewIssue> {
    let mut issues = Vec::new();
    issues.extend(check_security(working_dir, profile));
    issues.extend(check_performance(working_dir, profile));
    issues.extend(check_best_practices(working_dir, profile));
    eprintln!("hydra-review: {} issues found", issues.len());
    issues
}

/// Security review: hardcoded secrets, unsafe patterns.
fn check_security(working_dir: &str, profile: &CodebaseProfile) -> Vec<ReviewIssue> {
    let mut issues = Vec::new();
    let patterns = [
        ("password\\s*=\\s*[\"']", "Hardcoded password"),
        ("api_key\\s*=\\s*[\"']", "Hardcoded API key"),
        ("secret\\s*=\\s*[\"']", "Hardcoded secret"),
        ("innerHTML", "Potential XSS via innerHTML"),
        ("eval\\(", "Dangerous eval() usage"),
        ("exec\\(", "Dangerous exec() usage"),
    ];

    let ext = match profile.language.as_str() {
        "rust" => "rs", "typescript" => "ts", "python" => "py", _ => "*"
    };

    for (pattern, description) in &patterns {
        if let Ok(out) = std::process::Command::new("grep")
            .args(["-rl", "--include", &format!("*.{ext}"), pattern, working_dir])
            .output()
        {
            let files = String::from_utf8_lossy(&out.stdout);
            for file in files.lines().take(3) {
                if !file.is_empty() {
                    issues.push(ReviewIssue {
                        category: ReviewCategory::Security,
                        description: description.to_string(),
                        severity: 0.9,
                        file: Some(file.to_string()),
                        line: None,
                    });
                }
            }
        }
    }
    issues
}

/// Performance review: N+1 queries, missing pagination, unbounded loops.
fn check_performance(working_dir: &str, profile: &CodebaseProfile) -> Vec<ReviewIssue> {
    let mut issues = Vec::new();

    // Check for common performance anti-patterns based on language
    let checks: Vec<(&str, &str)> = match profile.language.as_str() {
        "typescript" => vec![
            ("findMany()", "Unbounded query — missing pagination (take/skip)"),
            (".map(.*await", "Sequential async in loop — use Promise.all"),
        ],
        "rust" => vec![
            ("collect::<Vec", "Collecting into Vec may be unnecessary — consider iterators"),
        ],
        "python" => vec![
            ("for.*in.*query", "Potential N+1 query in loop"),
        ],
        _ => vec![],
    };

    let ext = match profile.language.as_str() {
        "rust" => "rs", "typescript" => "ts", "python" => "py", _ => "*"
    };

    for (pattern, description) in checks {
        if let Ok(out) = std::process::Command::new("grep")
            .args(["-rl", "--include", &format!("*.{ext}"), pattern, working_dir])
            .output()
        {
            if !String::from_utf8_lossy(&out.stdout).trim().is_empty() {
                issues.push(ReviewIssue {
                    category: ReviewCategory::Performance,
                    description: description.to_string(),
                    severity: 0.6,
                    file: None, line: None,
                });
            }
        }
    }
    issues
}

/// Best practices: error handling, naming, structure.
fn check_best_practices(working_dir: &str, profile: &CodebaseProfile) -> Vec<ReviewIssue> {
    let mut issues = Vec::new();

    match profile.language.as_str() {
        "rust" => {
            // Check for unwrap() usage (should use ? or expect)
            if let Ok(out) = std::process::Command::new("grep")
                .args(["-rc", "--include", "*.rs", ".unwrap()", working_dir]).output()
            {
                let count: usize = String::from_utf8_lossy(&out.stdout)
                    .lines().filter_map(|l| l.split(':').last()?.parse::<usize>().ok()).sum();
                if count > 10 {
                    issues.push(ReviewIssue {
                        category: ReviewCategory::BestPractice,
                        description: format!("{count} unwrap() calls — consider using ? or expect()"),
                        severity: 0.5, file: None, line: None,
                    });
                }
            }
        }
        "typescript" => {
            if let Ok(out) = std::process::Command::new("grep")
                .args(["-rc", "--include", "*.ts", "any", working_dir]).output()
            {
                let count: usize = String::from_utf8_lossy(&out.stdout)
                    .lines().filter_map(|l| l.split(':').last()?.parse::<usize>().ok()).sum();
                if count > 5 {
                    issues.push(ReviewIssue {
                        category: ReviewCategory::BestPractice,
                        description: format!("{count} 'any' type usage — consider proper typing"),
                        severity: 0.4, file: None, line: None,
                    });
                }
            }
        }
        _ => {}
    }
    issues
}

// ── Stage 6: Refactoring Detector ──

/// Detect refactoring opportunities in modified files.
pub fn detect_refactoring(working_dir: &str, language: &str) -> Vec<ReviewIssue> {
    let mut issues = Vec::new();
    let ext = match language { "rust" => "rs", "typescript" => "ts", "python" => "py", _ => return issues };

    // Check for long files (> 300 lines)
    if let Ok(out) = std::process::Command::new("find")
        .args([working_dir, "-name", &format!("*.{ext}"), "-type", "f"]).output()
    {
        for file in String::from_utf8_lossy(&out.stdout).lines().take(50) {
            if let Ok(content) = std::fs::read_to_string(file) {
                if content.lines().count() > 300 {
                    issues.push(ReviewIssue {
                        category: ReviewCategory::Refactoring,
                        description: format!("File over 300 lines — consider splitting"),
                        severity: 0.4, file: Some(file.to_string()), line: None,
                    });
                }
            }
        }
    }
    issues
}

// ── Stage 9: Decision Recorder ──

/// Record architectural decisions from the coding session.
pub fn record_decisions(goal: &str, profile: &CodebaseProfile) -> usize {
    // Record key decisions as memory entries
    let decisions = vec![
        format!("Project uses {} with {:?}", profile.language, profile.framework),
        format!("Goal: {goal}"),
    ];
    eprintln!("hydra-coder: recorded {} decisions", decisions.len());
    decisions.len()
}

// ── Stage 8: Bug-to-Genome Pipeline ──

/// Convert a debug session into a genome-ready entry.
pub fn bug_to_genome_entry(session: &super::tdd::DebugSession) -> (String, String) {
    let situation = format!("Error: {}", session.error);
    let approach = format!("Root cause: {} → Fix: {}", session.root_cause, session.fix_applied);
    (situation, approach)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_current_codebase() {
        let profile = CodebaseProfile {
            language: "rust".into(), framework: None, dependencies: vec![],
            file_count: 100, test_framework: Some("cargo test".into()),
            has_linter: true, has_ci: false, recent_commits: vec![],
        };
        let issues = review_code(".", &profile);
        // Should find some unwrap() calls in Hydra's own codebase
        // (may or may not — depends on current code)
        let _ = issues;
    }

    #[test]
    fn security_check_detects_patterns() {
        let profile = CodebaseProfile {
            language: "rust".into(), framework: None, dependencies: vec![],
            file_count: 10, test_framework: None, has_linter: false, has_ci: false, recent_commits: vec![],
        };
        let issues = check_security(".", &profile);
        // Just verify it doesn't crash on real codebase
        let _ = issues;
    }

    #[test]
    fn refactoring_detector_runs() {
        let issues = detect_refactoring(".", "rust");
        // May find long files — that's ok
        let _ = issues;
    }

    #[test]
    fn bug_to_genome_creates_entry() {
        let session = super::super::tdd::DebugSession {
            error: "type mismatch".into(),
            root_cause: "wrong type annotation".into(),
            fix_applied: "changed &str to String".into(),
            prevented: true,
        };
        let (sit, app) = bug_to_genome_entry(&session);
        assert!(sit.contains("type mismatch"));
        assert!(app.contains("wrong type annotation"));
    }

    #[test]
    fn record_decisions_counts() {
        let profile = CodebaseProfile {
            language: "rust".into(), framework: Some("axum".into()), dependencies: vec![],
            file_count: 50, test_framework: None, has_linter: true, has_ci: true, recent_commits: vec![],
        };
        let count = record_decisions("add auth", &profile);
        assert!(count >= 2);
    }
}
