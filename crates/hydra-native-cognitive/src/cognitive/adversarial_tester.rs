//! Adversarial Self-Testing — prosecutor/defender/judge cycle on beliefs.
//! Runs in Dream State. Beliefs get refined and strengthened through challenge.
//!
//! Why isn't a sister doing this? Orchestrates LLM calls for the adversarial
//! debate. Uses Memory sister for storage of refined beliefs.

use crate::sisters::SistersHandle;
use hydra_native_state::operational_profile::ProfileBelief;

/// Result of an adversarial test on a single belief.
#[derive(Debug, Clone)]
pub struct AdversarialResult {
    pub original: String,
    pub prosecution: String,
    pub defense: String,
    pub refined_belief: String,
    pub original_confidence: f64,
    pub new_confidence: f64,
    pub new_edge_cases: Vec<String>,
    pub survived: bool,
}

/// Summary of a full adversarial testing session.
#[derive(Debug, Default)]
pub struct AdversarialReport {
    pub beliefs_tested: usize,
    pub beliefs_strengthened: usize,
    pub beliefs_weakened: usize,
    pub beliefs_refined: usize,
    pub new_edge_cases: usize,
}

impl AdversarialReport {
    pub fn summary(&self) -> String {
        format!(
            "Adversarial: {} tested, {} strengthened, {} weakened, {} refined, {} new edge cases",
            self.beliefs_tested, self.beliefs_strengthened,
            self.beliefs_weakened, self.beliefs_refined, self.new_edge_cases,
        )
    }
}

/// Run adversarial testing on a set of beliefs. Pick high-confidence beliefs
/// that haven't been tested recently. Uses Haiku for prosecutor/defender,
/// Sonnet for judge.
pub async fn test_beliefs(
    beliefs: &[ProfileBelief],
    sisters: &SistersHandle,
    llm_config: &hydra_model::LlmConfig,
    max_tests: usize,
) -> AdversarialReport {
    let mut report = AdversarialReport::default();

    // Select beliefs to test — prefer high confidence (more to lose)
    let mut candidates: Vec<&ProfileBelief> = beliefs.iter()
        .filter(|b| b.confidence >= 0.7)
        .collect();
    candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(max_tests);

    for belief in candidates {
        report.beliefs_tested += 1;

        let result = match test_single(belief, llm_config).await {
            Some(r) => r,
            None => continue,
        };

        // Store the refined belief
        if result.survived {
            if result.new_confidence > result.original_confidence {
                report.beliefs_strengthened += 1;
            }
            report.beliefs_refined += 1;
        } else {
            report.beliefs_weakened += 1;
        }
        report.new_edge_cases += result.new_edge_cases.len();

        // Store adversarial result in memory
        let content = format!(
            "[adversarial-tested] Original: {} | Refined: {} | Confidence: {:.0}% → {:.0}%",
            truncate(&result.original, 60),
            truncate(&result.refined_belief, 60),
            result.original_confidence * 100.0,
            result.new_confidence * 100.0,
        );
        sisters.memory_workspace_add(&content, &belief.topic).await;
    }

    eprintln!("[hydra:adversarial] {}", report.summary());
    report
}

/// Run the prosecutor/defender/judge cycle on a single belief.
async fn test_single(
    belief: &ProfileBelief,
    llm_config: &hydra_model::LlmConfig,
) -> Option<AdversarialResult> {
    // PROSECUTOR — find strongest counterargument (Haiku)
    let prosecution_prompt = format!(
        "Find the strongest counterargument to this belief:\n\
         \"{}\"\n\n\
         Be specific. Cite a concrete scenario where this belief fails.",
        belief.content,
    );
    let prosecution = crate::sisters::llm_micro_call(
        llm_config, &prosecution_prompt, "haiku",
    ).await?;

    // DEFENDER — defend the original belief (Haiku)
    let defense_prompt = format!(
        "Defend this belief: \"{}\"\n\n\
         Against this attack: \"{}\"\n\n\
         Acknowledge valid points but explain why the core belief holds.",
        belief.content, prosecution,
    );
    let defense = crate::sisters::llm_micro_call(
        llm_config, &defense_prompt, "haiku",
    ).await?;

    // JUDGE — synthesize refined belief with updated confidence (Sonnet)
    let judge_prompt = format!(
        "Belief: \"{}\"\nAttack: \"{}\"\nDefense: \"{}\"\n\n\
         Output exactly:\n\
         REFINED: <the refined belief incorporating valid points>\n\
         CONFIDENCE: <0.xx>\n\
         EDGE_CASES: <comma-separated new edge cases>\n\
         SURVIVED: <true/false>",
        belief.content, prosecution, defense,
    );
    let judgment = crate::sisters::llm_micro_call(
        llm_config, &judge_prompt, "sonnet",
    ).await?;

    // Parse judgment
    let refined = extract_field(&judgment, "REFINED:")
        .unwrap_or_else(|| belief.content.clone());
    let new_conf = extract_field(&judgment, "CONFIDENCE:")
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(belief.confidence);
    let edge_cases: Vec<String> = extract_field(&judgment, "EDGE_CASES:")
        .map(|s| s.split(',').map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect())
        .unwrap_or_default();
    let survived = extract_field(&judgment, "SURVIVED:")
        .map(|s| s.trim().to_lowercase().contains("true"))
        .unwrap_or(true);

    Some(AdversarialResult {
        original: belief.content.clone(),
        prosecution,
        defense,
        refined_belief: refined,
        original_confidence: belief.confidence,
        new_confidence: new_conf,
        new_edge_cases: edge_cases,
        survived,
    })
}

fn extract_field(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find(|l| l.trim().starts_with(prefix))
        .map(|l| l.trim()[prefix.len()..].trim().to_string())
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field() {
        let text = "REFINED: Updated belief\nCONFIDENCE: 0.88\nSURVIVED: true";
        assert_eq!(extract_field(text, "REFINED:"), Some("Updated belief".into()));
        assert_eq!(extract_field(text, "CONFIDENCE:"), Some("0.88".into()));
    }

    #[test]
    fn test_report_summary() {
        let report = AdversarialReport {
            beliefs_tested: 5,
            beliefs_strengthened: 3,
            beliefs_weakened: 1,
            beliefs_refined: 4,
            new_edge_cases: 7,
        };
        let s = report.summary();
        assert!(s.contains("5 tested"));
        assert!(s.contains("3 strengthened"));
    }
}
