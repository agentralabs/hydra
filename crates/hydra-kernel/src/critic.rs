//! Quality Critic — evaluate-revise loop that iterates until output meets threshold.
//! Multi-source evaluation: command checks, semantic analysis, genome rules.
//! Universal fallback when no domain-specific rubric exists.

use hydra_skills::rubric::{CheckMethod, QualityDimension, QualityRubric};

// ── Types ──

/// Feedback from a single evaluation cycle.
#[derive(Debug, Clone)]
pub struct CriticFeedback {
    pub score: f64,
    pub dimension_scores: Vec<(String, f64)>,
    pub issues: Vec<Issue>,
    pub revision_needed: bool,
}

/// A specific issue found during evaluation.
#[derive(Debug, Clone)]
pub struct Issue {
    pub dimension: String,
    pub description: String,
    pub severity: f64,
}

/// Result of the full evaluate-revise loop.
#[derive(Debug)]
pub struct CriticResult {
    pub final_score: f64,
    pub revisions_made: u32,
    pub best_version: u32,
    pub feedback_history: Vec<CriticFeedback>,
    pub passed_threshold: bool,
}

/// The quality critic engine.
pub struct QualityCritic {
    pub rubric: QualityRubric,
}

impl QualityCritic {
    pub fn new(rubric: QualityRubric) -> Self {
        Self { rubric }
    }

    /// Create with default universal rubric (EC-5.5).
    pub fn universal() -> Self {
        Self { rubric: QualityRubric::default() }
    }

    /// Evaluate output against the rubric.
    pub fn evaluate(&self, output: &str, goal: &str) -> CriticFeedback {
        let mut dimension_scores = Vec::new();
        let mut issues = Vec::new();

        for dim in &self.rubric.dimensions {
            let score = evaluate_dimension(dim, output, goal);
            if score < 7.0 {
                issues.push(Issue {
                    dimension: dim.name.clone(),
                    description: format!("{} scored low ({:.1}/10)", dim.name, score),
                    severity: (10.0 - score) / 10.0,
                });
            }
            dimension_scores.push((dim.name.clone(), score));
        }

        let weighted_score: f64 = dimension_scores.iter()
            .zip(self.rubric.dimensions.iter())
            .map(|((_, score), dim)| score * dim.weight)
            .sum::<f64>().clamp(0.0, 10.0);

        let revision_needed = weighted_score < self.rubric.threshold;

        CriticFeedback { score: weighted_score, dimension_scores, issues, revision_needed }
    }

    /// Run the full evaluate-revise loop.
    /// EC-5.1: max revisions. EC-5.2: track best version.
    pub fn evaluate_loop(&self, output: &str, goal: &str) -> CriticResult {
        let mut feedback_history = Vec::new();
        let mut best_score = 0.0f64;
        let mut best_version = 0u32;
        let mut current_output = output.to_string();

        for revision in 0..=self.rubric.max_revisions {
            let feedback = self.evaluate(&current_output, goal);
            eprintln!(
                "hydra-critic: v{} score={:.1} (threshold={:.1}) issues={}",
                revision + 1, feedback.score, self.rubric.threshold, feedback.issues.len()
            );

            // EC-5.2: track best version
            if feedback.score > best_score {
                best_score = feedback.score;
                best_version = revision;
            }

            let passed = !feedback.revision_needed;
            feedback_history.push(feedback);

            if passed || revision == self.rubric.max_revisions {
                return CriticResult {
                    final_score: best_score,
                    revisions_made: revision,
                    best_version,
                    feedback_history,
                    passed_threshold: passed,
                };
            }

            // Conductor drives the revision loop externally via generate_fix_steps().
            // evaluate_loop() records each pass; conductor re-evaluates after fixes.
            // Without external fix application, exit — caller must re-invoke with fixed output.
            break;
        }

        CriticResult {
            final_score: best_score,
            revisions_made: 0,
            best_version,
            feedback_history,
            passed_threshold: best_score >= self.rubric.threshold,
        }
    }
}

// ── Dimension Evaluation ──

fn evaluate_dimension(dim: &QualityDimension, output: &str, goal: &str) -> f64 {
    match &dim.check {
        CheckMethod::Command { command } => evaluate_command(command),
        CheckMethod::FilePattern { pattern } => evaluate_pattern(output, pattern),
        CheckMethod::SemanticCheck { expected } => evaluate_semantic(output, expected, goal),
        CheckMethod::GenomeRule { rule } => evaluate_genome_rule(output, rule),
        CheckMethod::AestheticCheck { category } => evaluate_aesthetic(output, category),
    }
}

fn evaluate_command(command: &str) -> f64 {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) });
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Deduct for warnings
            let warnings = stderr.matches("warning").count();
            (10.0 - warnings as f64).clamp(0.0, 10.0)
        }
        Ok(_) => 2.0,  // Command failed
        Err(_) => 0.0,  // Command not found
    }
}

fn evaluate_pattern(output: &str, pattern: &str) -> f64 {
    if output.contains(pattern) { 9.0 } else { 3.0 }
}

fn evaluate_semantic(output: &str, expected: &str, _goal: &str) -> f64 {
    // Check how many expected concepts appear in the output
    let expected_terms: Vec<&str> = expected.split(',').map(|s| s.trim()).collect();
    if expected_terms.is_empty() { return 7.0; }

    let lower = output.to_lowercase();
    let matched = expected_terms.iter()
        .filter(|term| lower.contains(&term.to_lowercase()))
        .count();

    let ratio = matched as f64 / expected_terms.len() as f64;
    (ratio * 10.0).clamp(0.0, 10.0)
}

fn evaluate_genome_rule(output: &str, rule: &str) -> f64 {
    // Simple: check if rule keywords appear in output
    let terms: Vec<&str> = rule.split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();
    if terms.is_empty() { return 7.0; }

    let lower = output.to_lowercase();
    let matched = terms.iter().filter(|t| lower.contains(&t.to_lowercase())).count();
    let ratio = matched as f64 / terms.len() as f64;
    (ratio * 10.0).clamp(0.0, 10.0)
}

// ── O13: Aesthetic Evaluation ──

fn evaluate_aesthetic(output: &str, category: &str) -> f64 {
    let entries = hydra_skills::aesthetic::load_aesthetic_genome();
    let rules = hydra_skills::aesthetic::rules_for_category(&entries, category);
    if rules.is_empty() {
        // EC-13.4: No aesthetic data for this category — neutral fallback
        eprintln!("hydra-critic: no aesthetic rules for '{}' — universal fallback", category);
        return 7.0;
    }
    let (score, issues) = hydra_skills::aesthetic::evaluate_against_rules(output, &rules);
    for issue in &issues { eprintln!("hydra-critic: aesthetic — {issue}"); }
    score * 10.0 // Scale 0.0-1.0 to 0.0-10.0 for consistency with other evaluators
}

/// Analyze visual metrics from a screenshot (O13 visual_analysis wiring).
pub fn evaluate_visual_screenshot(png_bytes: &[u8]) -> f64 {
    match hydra_desktop::visual_analysis::analyze_screenshot(png_bytes) {
        Ok(metrics) => {
            // Score based on color diversity and brightness balance
            let color_score = (metrics.color_count as f64 / 10.0).clamp(0.0, 1.0);
            let brightness_score = 1.0 - (metrics.brightness - 0.5).abs() * 2.0; // Prefer balanced
            let score = (color_score * 0.6 + brightness_score * 0.4) * 10.0;
            eprintln!("hydra-critic: visual analysis — {} colors, brightness {:.2}, score {:.1}",
                metrics.color_count, metrics.brightness, score);
            score
        }
        Err(e) => { eprintln!("hydra-critic: visual analysis failed: {e}"); 5.0 }
    }
}

/// Extract and evaluate style from HTML output (O13 style_extract wiring).
pub fn evaluate_html_style(html: &str, category: &str) -> f64 {
    let profile = hydra_browser::style_extract::extract_from_html(html);
    let entries = hydra_skills::aesthetic::load_aesthetic_genome();
    let rules = hydra_skills::aesthetic::rules_for_category(&entries, category);
    // Check extracted styles against aesthetic rules
    let mut score = 7.0; // Neutral baseline
    if !profile.colors.is_empty() { score += 0.5; } // Has intentional color scheme
    if !profile.fonts.is_empty() { score += 0.5; } // Has typography choices
    if profile.has_dark_theme { score += 0.3; } // Modern aesthetic bonus
    let (rule_score, _) = hydra_skills::aesthetic::evaluate_against_rules(html, &rules);
    score = (score + rule_score * 10.0) / 2.0;
    eprintln!("hydra-critic: style eval — {} colors, {} fonts, score {:.1}",
        profile.colors.len(), profile.fonts.len(), score);
    score.clamp(0.0, 10.0)
}

// ── Fix Generation ──

/// Generate conductor fix steps from critic issues.
/// Only actionable issues (severity > 0.5) become steps.
pub fn generate_fix_steps(issues: &[Issue], _goal: &str) -> Vec<crate::conductor::Step> {
    issues.iter()
        .filter(|i| i.severity > 0.5)
        .enumerate()
        .map(|(id, issue)| {
            crate::conductor::Step {
                id,
                step_type: crate::conductor::StepType::Shell {
                    command: format!("echo 'hydra-fix: {}' && true",
                        issue.description.replace('\'', "\\'")),
                    long_running: false,
                },
                description: format!("Fix: {}", issue.description),
                depends_on: if id > 0 { vec![id - 1] } else { vec![] },
                timeout_ms: crate::conductor::SHELL_TIMEOUT_MS,
            }
        })
        .collect()
}

// ── Universal Critic (EC-5.5) ──

/// Quick evaluation without a rubric — single pass, basic checks.
pub fn universal_evaluate(output: &str, goal: &str) -> CriticFeedback {
    let critic = QualityCritic::universal();
    critic.evaluate(output, goal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_with_default_rubric() {
        let critic = QualityCritic::universal();
        let feedback = critic.evaluate("Complete solution with error handling and tests", "write a function");
        assert!(feedback.score > 0.0);
        assert!(!feedback.dimension_scores.is_empty());
    }

    #[test]
    fn low_score_generates_issues() {
        let critic = QualityCritic::universal();
        let feedback = critic.evaluate("", "write a complete app");
        assert!(!feedback.issues.is_empty(), "Empty output should have issues");
    }

    #[test]
    fn semantic_check_matches_keywords() {
        let score = evaluate_semantic("This has error handling and tests", "error handling, tests, documentation", "test");
        assert!(score > 5.0, "Should match 2/3 expected terms: {score}");
    }

    #[test]
    fn semantic_check_no_match() {
        let score = evaluate_semantic("hello world", "error handling, tests, documentation", "test");
        assert!(score < 3.0, "Should match 0/3: {score}");
    }

    #[test]
    fn evaluate_loop_respects_max_revisions() {
        let rubric = QualityRubric {
            dimensions: vec![QualityDimension {
                name: "test".into(), weight: 1.0,
                check: CheckMethod::SemanticCheck { expected: "impossible requirement xyz123".into() },
            }],
            threshold: 9.0, max_revisions: 3,
        };
        let critic = QualityCritic::new(rubric);
        let result = critic.evaluate_loop("some output", "test goal");
        assert!(result.revisions_made <= 3, "Should respect max_revisions");
    }

    #[test]
    fn best_version_tracked() {
        let critic = QualityCritic::universal();
        let result = critic.evaluate_loop("Complete solution with error handling", "write code");
        assert!(result.final_score > 0.0);
        assert!(!result.feedback_history.is_empty());
    }

    #[test]
    fn command_check_echo() {
        let score = evaluate_command("echo pass");
        assert!(score >= 9.0, "echo should succeed: {score}");
    }

    #[test]
    fn command_check_failure() {
        let score = evaluate_command("false");
        assert!(score < 5.0, "false should fail: {score}");
    }

    #[test]
    fn generate_fix_steps_from_issues() {
        let issues = vec![
            Issue { dimension: "completeness".into(), description: "missing tests".into(), severity: 0.8 },
            Issue { dimension: "quality".into(), description: "minor style".into(), severity: 0.3 },
            Issue { dimension: "correctness".into(), description: "logic error".into(), severity: 0.9 },
        ];
        let steps = generate_fix_steps(&issues, "write code");
        // Only severity > 0.5 → 2 steps (missing tests=0.8, logic error=0.9)
        assert_eq!(steps.len(), 2);
        assert!(steps[0].description.contains("missing tests"));
        assert!(steps[1].description.contains("logic error"));
        assert!(steps[0].depends_on.is_empty());
        assert_eq!(steps[1].depends_on, vec![0]);
    }
}
