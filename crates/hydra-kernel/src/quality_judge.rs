//! O32: Quality Judgment — evaluates whether completed work actually meets the goal.
//!
//! After task completion, takes the original goal + final artifacts + screenshot
//! and evaluates: was this done RIGHT? Not just "did it finish" — but "is it correct?"
//! Like a human reviewing their own work before submitting.

/// A single quality criterion and whether it was met.
#[derive(Debug, Clone)]
pub struct QualityCriterion {
    pub check: String,
    pub met: bool,
    pub evidence: String,
    pub weight: f64,
}

/// Overall quality verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum QualityVerdict {
    Excellent,    // >= 0.95
    Good,         // >= 0.80
    Incomplete,   // >= 0.50
    Failed,       // < 0.50
}

impl QualityVerdict {
    pub fn label(&self) -> &str {
        match self {
            Self::Excellent => "EXCELLENT",
            Self::Good => "GOOD",
            Self::Incomplete => "INCOMPLETE",
            Self::Failed => "FAILED",
        }
    }
    pub fn from_score(score: f64) -> Self {
        if score >= 0.95 { Self::Excellent }
        else if score >= 0.80 { Self::Good }
        else if score >= 0.50 { Self::Incomplete }
        else { Self::Failed }
    }
}

/// Complete quality report for a finished task.
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub goal: String,
    pub criteria: Vec<QualityCriterion>,
    pub overall_score: f64,
    pub verdict: QualityVerdict,
    pub remediation: Option<String>,
}

/// Artifacts from a completed task.
#[derive(Debug, Clone)]
pub struct TaskArtifacts {
    pub files_created: Vec<String>,
    pub step_history: Vec<(String, String)>,
    pub duration_ms: u64,
    pub final_screen_description: String,
}

/// Evaluate task output against the original goal.
pub fn evaluate(
    goal: &str,
    artifacts: &TaskArtifacts,
    genome: &hydra_genome::GenomeStore,
) -> QualityReport {
    let criteria = derive_criteria(goal, artifacts);
    let checked = check_criteria(&criteria, artifacts);
    let (verdict, remediation) = judge(&checked);

    let overall_score = if checked.is_empty() { 0.5 } else {
        let total_weight: f64 = checked.iter().map(|c| c.weight).sum();
        let met_weight: f64 = checked.iter().filter(|c| c.met).map(|c| c.weight).sum();
        if total_weight > 0.0 { met_weight / total_weight } else { 0.5 }
    };

    eprintln!("hydra-quality: goal='{}' score={:.0}% verdict={} criteria={}",
        &goal[..goal.len().min(50)], overall_score * 100.0, verdict.label(), checked.len());

    // Record quality assessment in genome for learning
    let tag = format!("quality:{}", &goal[..goal.len().min(80)]);
    let _ = genome.query(&tag); // Touch for future matching

    QualityReport { goal: goal.into(), criteria: checked, overall_score, verdict, remediation }
}

/// Derive quality criteria from the goal statement.
fn derive_criteria(goal: &str, artifacts: &TaskArtifacts) -> Vec<QualityCriterion> {
    let mut criteria = Vec::new();
    let lower = goal.to_lowercase();

    // Universal criteria (apply to every task)
    criteria.push(QualityCriterion {
        check: "Task completed without errors".into(),
        met: false, evidence: String::new(), weight: 0.3,
    });
    criteria.push(QualityCriterion {
        check: "Output artifacts exist".into(),
        met: false, evidence: String::new(), weight: 0.2,
    });

    // Goal-specific criteria extracted from keywords
    extract_numeric_criteria(&lower, &mut criteria);
    extract_domain_criteria(&lower, &mut criteria);

    // History-based: did all steps complete?
    criteria.push(QualityCriterion {
        check: format!("All {} steps completed", artifacts.step_history.len()),
        met: false, evidence: String::new(), weight: 0.2,
    });

    criteria
}

/// Extract criteria from numbers in the goal (e.g., "2-bedroom" → check for 2 bedrooms).
fn extract_numeric_criteria(goal: &str, criteria: &mut Vec<QualityCriterion>) {
    // Simple pattern: "N X" where N is a number and X is a noun
    let words: Vec<&str> = goal.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if let Ok(n) = word.parse::<u32>() {
            if let Some(noun) = words.get(i + 1) {
                criteria.push(QualityCriterion {
                    check: format!("Contains {n} {noun}"),
                    met: false, evidence: String::new(), weight: 0.3,
                });
            }
        }
    }
}

/// Extract criteria from domain keywords.
fn extract_domain_criteria(goal: &str, criteria: &mut Vec<QualityCriterion>) {
    if goal.contains("floor plan") || goal.contains("layout") {
        criteria.push(QualityCriterion {
            check: "Walls form closed perimeter".into(),
            met: false, evidence: String::new(), weight: 0.2,
        });
        criteria.push(QualityCriterion {
            check: "Doors placed at wall openings".into(),
            met: false, evidence: String::new(), weight: 0.15,
        });
    }
    if goal.contains("report") || goal.contains("document") {
        criteria.push(QualityCriterion {
            check: "Document has title and sections".into(),
            met: false, evidence: String::new(), weight: 0.2,
        });
    }
    if goal.contains("email") || goal.contains("message") {
        criteria.push(QualityCriterion {
            check: "Message has subject, greeting, and body".into(),
            met: false, evidence: String::new(), weight: 0.2,
        });
    }
}

/// Check criteria against actual artifacts.
fn check_criteria(criteria: &[QualityCriterion], artifacts: &TaskArtifacts) -> Vec<QualityCriterion> {
    criteria.iter().map(|c| {
        let (met, evidence) = match c.check.as_str() {
            "Task completed without errors" => {
                let no_errors = !artifacts.step_history.iter()
                    .any(|(_, obs)| obs.to_lowercase().contains("error") || obs.to_lowercase().contains("fail"));
                (no_errors, if no_errors { "No errors in step history".into() }
                    else { "Errors found in step history".into() })
            }
            "Output artifacts exist" => {
                let has = !artifacts.files_created.is_empty()
                    || !artifacts.final_screen_description.is_empty();
                (has, format!("{} files, screen: {}",
                    artifacts.files_created.len(),
                    &artifacts.final_screen_description[..artifacts.final_screen_description.len().min(50)]))
            }
            s if s.starts_with("All ") && s.ends_with(" steps completed") => {
                let done = artifacts.step_history.last()
                    .map(|(_, o)| o.to_lowercase().contains("done") || o.to_lowercase().contains("complete"))
                    .unwrap_or(false);
                (done, format!("{} steps executed", artifacts.step_history.len()))
            }
            _ => {
                // For derived criteria, check if the screen description mentions them
                let met = artifacts.final_screen_description.to_lowercase()
                    .contains(&c.check.to_lowercase());
                (met, if met { "Found in final state".into() } else { "Not found in final state".into() })
            }
        };
        QualityCriterion { check: c.check.clone(), met, evidence, weight: c.weight }
    }).collect()
}

/// Determine verdict and remediation from checked criteria.
fn judge(criteria: &[QualityCriterion]) -> (QualityVerdict, Option<String>) {
    if criteria.is_empty() { return (QualityVerdict::Good, None); }
    let total_weight: f64 = criteria.iter().map(|c| c.weight).sum();
    let met_weight: f64 = criteria.iter().filter(|c| c.met).map(|c| c.weight).sum();
    let score = if total_weight > 0.0 { met_weight / total_weight } else { 0.5 };
    let verdict = QualityVerdict::from_score(score);

    let remediation = if verdict == QualityVerdict::Incomplete || verdict == QualityVerdict::Failed {
        let unmet: Vec<&str> = criteria.iter().filter(|c| !c.met).map(|c| c.check.as_str()).collect();
        Some(format!("Fix before done: {}", unmet.join(", ")))
    } else { None };

    (verdict, remediation)
}
