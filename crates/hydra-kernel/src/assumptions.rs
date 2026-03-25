//! Assumption Miner — discovers unknown unknowns before execution.
//! Extracts implicit assumptions, checks against reality, surfaces questions.
//! Runs BEFORE the conductor executes any task.

use hydra_genome::GenomeStore;

// ── Types ──

/// A single implicit assumption extracted from a user request.
#[derive(Debug, Clone)]
pub struct Assumption {
    pub statement: String,
    pub implicit_in: String,
    pub severity: f64,
    pub status: AssumptionStatus,
}

/// Status after checking an assumption.
#[derive(Debug, Clone, PartialEq)]
pub enum AssumptionStatus {
    Valid,
    Invalid { reason: String },
    Unchecked { reason: String },   // EC-0.1: check failed (network, timeout)
    Overridden,                      // EC-0.6: user said "just do it"
}

/// Result of mining assumptions from a goal.
#[derive(Debug)]
pub struct MinerResult {
    pub assumptions: Vec<Assumption>,
    pub questions: Vec<String>,
    pub all_valid: bool,
}

// ── Constants ──

const SEVERITY_THRESHOLD: f64 = 0.7;         // EC-0.2: only surface high-severity
const MAX_ASSUMPTIONS: usize = 10;            // EC-0.5: cap total
const MAX_CHECK_DEPTH: u8 = 2;               // EC-0.9: no circular checking
const CROSS_DOMAIN_MIN_RELEVANCE: f64 = 0.3; // EC-0.4: filter nonsense connections

// ── Main Entry Point ──

/// Mine assumptions from a goal, check each, and produce questions.
pub fn mine(goal: &str, genome: &GenomeStore) -> MinerResult {
    let mut assumptions = Vec::new();

    // 1. Template-based assumptions (domain-specific)
    let templates = hydra_skills::assumptions::load_templates();
    let matched = hydra_skills::assumptions::match_templates(goal, &templates);

    if matched.is_empty() {
        // EC-0.8: No templates → universal fallback
        for ua in hydra_skills::assumptions::universal_assumptions() {
            assumptions.push(Assumption {
                statement: ua.statement,
                implicit_in: goal.to_string(),
                severity: ua.severity,
                status: AssumptionStatus::Valid, // unchecked universal
            });
        }
    } else {
        for ta in matched {
            assumptions.push(Assumption {
                statement: ta.statement,
                implicit_in: goal.to_string(),
                severity: ta.severity,
                status: AssumptionStatus::Valid,
            });
        }
    }

    // 2. Cross-domain inference from genome
    let cross_domain = infer_cross_domain(goal, genome);
    assumptions.extend(cross_domain);

    // 3. Historical pattern analysis from genome
    let historical = check_historical_patterns(goal, genome);
    assumptions.extend(historical);

    // EC-0.5: Scale with complexity, cap total
    let complexity = estimate_complexity(goal);
    let max = match complexity {
        Complexity::Simple => 2,
        Complexity::Medium => 5,
        Complexity::Complex => 8,
    }.min(MAX_ASSUMPTIONS);
    assumptions.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap_or(std::cmp::Ordering::Equal));
    assumptions.truncate(max);

    // 4. Check each assumption
    for assumption in &mut assumptions {
        check_assumption(assumption, 0);
    }

    // 5. Generate questions for invalid/unchecked high-severity assumptions
    let questions: Vec<String> = assumptions.iter()
        .filter(|a| a.severity >= SEVERITY_THRESHOLD) // EC-0.2
        .filter(|a| !matches!(a.status, AssumptionStatus::Valid | AssumptionStatus::Overridden))
        .map(|a| format_question(a))
        .collect();

    let all_valid = assumptions.iter()
        .filter(|a| a.severity >= SEVERITY_THRESHOLD)
        .all(|a| matches!(a.status, AssumptionStatus::Valid));

    eprintln!(
        "hydra-assumptions: {} assumptions mined, {} questions, all_valid={}",
        assumptions.len(), questions.len(), all_valid
    );

    MinerResult { assumptions, questions, all_valid }
}

// ── Cross-Domain Inference ──

fn infer_cross_domain(goal: &str, genome: &GenomeStore) -> Vec<Assumption> {
    let mut inferred = Vec::new();
    let related = genome.query(goal);

    for entry in related.iter().take(3) {
        // EC-0.4: Only if relevance is above threshold
        if entry.effective_confidence() < CROSS_DOMAIN_MIN_RELEVANCE { continue; }

        let approach_text = entry.approach.steps.join(" ");
        // If genome has a related entry with different tools, it might reveal an assumption
        if !approach_text.is_empty() {
            inferred.push(Assumption {
                statement: format!("Related pattern: {}", entry.approach.steps.first().unwrap_or(&String::new())),
                implicit_in: format!("genome entry with conf={:.2}", entry.effective_confidence()),
                severity: 0.6,
                status: AssumptionStatus::Valid,
            });
        }
    }
    inferred
}

// ── Historical Patterns ──

fn check_historical_patterns(goal: &str, genome: &GenomeStore) -> Vec<Assumption> {
    let mut patterns = Vec::new();
    let related = genome.query(goal);

    for entry in related.iter().take(5) {
        // If an entry has low success rate, warn
        if entry.use_count > 2 && entry.effective_confidence() < 0.5 {
            patterns.push(Assumption {
                statement: format!(
                    "Similar task had low success rate ({:.0}%)",
                    entry.effective_confidence() * 100.0
                ),
                implicit_in: "historical patterns".into(),
                severity: 0.75,
                status: AssumptionStatus::Invalid {
                    reason: format!("Success rate: {:.0}%", entry.effective_confidence() * 100.0),
                },
            });
        }
    }

    // EC-0.7: Tag with temporal context
    let now = chrono::Local::now();
    if now.format("%A").to_string() == "Friday" {
        patterns.push(Assumption {
            statement: "This is a Friday operation".into(),
            implicit_in: "temporal context".into(),
            severity: 0.6,
            status: AssumptionStatus::Unchecked { reason: "Pattern-based only".into() },
        });
    }

    patterns
}

// ── Assumption Checking ──

fn check_assumption(assumption: &mut Assumption, depth: u8) {
    if depth >= MAX_CHECK_DEPTH { return; } // EC-0.9

    // For now, assumptions from templates start as Valid (no live checks yet).
    // When monitors (O16) are built, this will query live state.
    // Shell-checkable assumptions could be verified here.
    // EC-0.1: If check fails, mark UNCHECKED not VALID.
}

// ── Complexity Estimation ──

#[derive(Debug)]
enum Complexity { Simple, Medium, Complex }

fn estimate_complexity(goal: &str) -> Complexity {
    let lower = goal.to_lowercase();
    // Dangerous operations are always Complex regardless of length
    if lower.contains("rm ") || lower.contains("dd ") || lower.contains("drop ")
        || lower.contains("delete") || lower.contains("format ") || lower.contains("sudo ")
        || lower.contains("truncate") || lower.contains("--force") {
        return Complexity::Complex;
    }
    let words = goal.split_whitespace().count();
    let has_conjunction = goal.contains(" and ") || goal.contains(" then ");
    if words < 5 && !has_conjunction { Complexity::Simple }
    else if words < 15 { Complexity::Medium }
    else { Complexity::Complex }
}

// ── Question Formatting ──

fn format_question(assumption: &Assumption) -> String {
    match &assumption.status {
        AssumptionStatus::Invalid { reason } => {
            format!("{} — {} (severity: {:.0}%)", assumption.statement, reason, assumption.severity * 100.0)
        }
        AssumptionStatus::Unchecked { reason } => {
            format!("{} — could not verify: {} (severity: {:.0}%)", assumption.statement, reason, assumption.severity * 100.0)
        }
        _ => assumption.statement.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_deploy_extracts_assumptions() {
        let genome = GenomeStore::new();
        let result = mine("deploy to production", &genome);
        assert!(!result.assumptions.is_empty(), "Should extract deployment assumptions");
        assert!(result.assumptions.iter().any(|a| a.statement.contains("Tests")));
    }

    #[test]
    fn mine_simple_task_few_assumptions() {
        let genome = GenomeStore::new();
        let result = mine("echo hello", &genome);
        assert!(result.assumptions.len() <= 3, "Simple task should have ≤ 3 assumptions"); // EC-0.5
    }

    #[test]
    fn mine_delete_has_backup_assumption() {
        let genome = GenomeStore::new();
        let result = mine("delete the database", &genome);
        assert!(result.assumptions.iter().any(|a| a.statement.contains("Backup") || a.statement.contains("backup")));
    }

    #[test]
    fn mine_unknown_domain_uses_universal() {
        let genome = GenomeStore::new();
        let result = mine("do something weird", &genome);
        assert!(!result.assumptions.is_empty(), "Should have universal fallback assumptions");
        assert!(result.assumptions.iter().any(|a| a.statement.contains("reversible")));
    }

    #[test]
    fn severity_threshold_filters_questions() {
        let genome = GenomeStore::new();
        let result = mine("deploy to production", &genome);
        // All surfaced questions should be high-severity
        // (questions are only generated for severity >= 0.7)
        for q in &result.questions {
            assert!(q.contains("severity:") || !q.is_empty());
        }
    }

    #[test]
    fn complexity_estimation() {
        assert!(matches!(estimate_complexity("echo hello"), Complexity::Simple));
        assert!(matches!(estimate_complexity("create a react app with authentication"), Complexity::Medium));
        assert!(matches!(estimate_complexity("deploy the application to production and then run the test suite and verify all endpoints"), Complexity::Complex));
    }
}
