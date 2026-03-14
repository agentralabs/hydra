//! Reasoning Verifier — checks every important response against beliefs,
//! history, and logical coherence BEFORE the user sees it.
//!
//! Why isn't a sister doing this? Veritas handles claim verification;
//! this module orchestrates a full verification chain across multiple
//! sisters and belief stores.

use hydra_native_state::operational_profile::ProfileBelief;

/// Result of the verification chain.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub original_response: String,
    pub verified: bool,
    pub issues: Vec<VerificationIssue>,
    pub confidence_calibrated: bool,
    pub modified_response: Option<String>,
}

/// A specific issue found during verification.
#[derive(Debug, Clone)]
pub struct VerificationIssue {
    pub check_type: VerificationCheck,
    pub severity: IssueSeverity,
    pub description: String,
    pub suggestion: String,
}

/// Type of verification check performed.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationCheck {
    BeliefContradiction,
    HistoricalRejection,
    LogicalIncoherence,
    OverConfidence,
    UnderConfidence,
}

/// How serious the issue is.
#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Critical,  // Must be fixed before showing to user
    Warning,   // Should be flagged but can be shown
    Info,      // FYI — minor concern
}

/// Verify a response against the full verification chain.
pub fn verify_response(
    response: &str,
    beliefs: &[ProfileBelief],
    history: &[(String, String)],
) -> VerificationResult {
    let mut issues = Vec::new();

    // Step 1: Check belief consistency
    check_belief_consistency(response, beliefs, &mut issues);

    // Step 2: Check historical rejection patterns
    check_historical_consistency(response, history, &mut issues);

    // Step 3: Check confidence calibration
    check_confidence_calibration(response, beliefs, &mut issues);

    let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
    let confidence_calibrated = !issues.iter().any(|i| matches!(
        i.check_type, VerificationCheck::OverConfidence | VerificationCheck::UnderConfidence
    ));

    let modified = if has_critical {
        Some(build_modified_response(response, &issues))
    } else {
        None
    };

    VerificationResult {
        original_response: response.to_string(),
        verified: !has_critical,
        issues,
        confidence_calibrated,
        modified_response: modified,
    }
}

/// Check if response contradicts any loaded beliefs.
fn check_belief_consistency(
    response: &str,
    beliefs: &[ProfileBelief],
    issues: &mut Vec<VerificationIssue>,
) {
    let response_lower = response.to_lowercase();

    for belief in beliefs {
        if belief.confidence < 0.8 {
            continue; // Only check high-confidence beliefs
        }

        // Check for direct contradiction indicators
        let belief_lower = belief.content.to_lowercase();
        let belief_keywords: Vec<&str> = belief_lower.split_whitespace()
            .filter(|w| w.len() >= 5)
            .collect();

        // If response mentions the same topic but with negation
        let topic_match = belief_keywords.iter()
            .filter(|w| response_lower.contains(*w))
            .count();

        if topic_match >= 2 {
            // Check for negation patterns near belief keywords
            let has_negation = response_lower.contains("never")
                || response_lower.contains("don't")
                || response_lower.contains("avoid")
                || response_lower.contains("shouldn't");

            let belief_is_positive = !belief_lower.contains("never")
                && !belief_lower.contains("don't")
                && !belief_lower.contains("avoid");

            if has_negation && belief_is_positive {
                issues.push(VerificationIssue {
                    check_type: VerificationCheck::BeliefContradiction,
                    severity: IssueSeverity::Warning,
                    description: format!(
                        "Response may contradict belief: '{}'",
                        truncate(&belief.content, 80),
                    ),
                    suggestion: format!(
                        "Consider: profile belief states '{}' (confidence: {:.0}%)",
                        truncate(&belief.content, 60), belief.confidence * 100.0,
                    ),
                });
            }
        }
    }
}

/// Check if similar advice was rejected in past interactions.
fn check_historical_consistency(
    response: &str,
    history: &[(String, String)],
    issues: &mut Vec<VerificationIssue>,
) {
    let response_lower = response.to_lowercase();

    // Look for correction patterns in history
    for (user_msg, _hydra_response) in history {
        let msg_lower = user_msg.to_lowercase();
        let is_correction = msg_lower.contains("no, ")
            || msg_lower.contains("that's wrong")
            || msg_lower.contains("actually,")
            || msg_lower.contains("i disagree");

        if !is_correction {
            continue;
        }

        // Check if the current response is similar to what was corrected
        let correction_words: Vec<&str> = msg_lower.split_whitespace()
            .filter(|w| w.len() >= 4)
            .collect();

        let overlap = correction_words.iter()
            .filter(|w| response_lower.contains(*w))
            .count();

        if overlap >= 3 {
            issues.push(VerificationIssue {
                check_type: VerificationCheck::HistoricalRejection,
                severity: IssueSeverity::Warning,
                description: "Similar response was corrected by user previously".into(),
                suggestion: format!(
                    "User previously said: '{}'",
                    truncate(user_msg, 80),
                ),
            });
        }
    }
}

/// Check if response confidence matches belief evidence strength.
fn check_confidence_calibration(
    response: &str,
    beliefs: &[ProfileBelief],
    issues: &mut Vec<VerificationIssue>,
) {
    let response_lower = response.to_lowercase();

    // Detect overly confident language
    let confident_phrases = ["definitely", "certainly", "absolutely", "always", "guaranteed"];
    let uncertain_evidence = beliefs.iter()
        .any(|b| b.confidence < 0.7);

    let uses_confident_language = confident_phrases.iter()
        .any(|p| response_lower.contains(p));

    if uses_confident_language && uncertain_evidence {
        issues.push(VerificationIssue {
            check_type: VerificationCheck::OverConfidence,
            severity: IssueSeverity::Info,
            description: "Response uses confident language but evidence has uncertainty".into(),
            suggestion: "Consider softening to 'likely', 'based on available evidence'".into(),
        });
    }

    // Detect under-confidence when beliefs are strong
    let hedging_phrases = ["maybe", "i'm not sure", "hard to say", "it depends"];
    let strong_evidence = beliefs.iter()
        .filter(|b| b.confidence >= 0.9)
        .count() >= 3;

    let uses_hedging = hedging_phrases.iter()
        .any(|p| response_lower.contains(p));

    if uses_hedging && strong_evidence {
        issues.push(VerificationIssue {
            check_type: VerificationCheck::UnderConfidence,
            severity: IssueSeverity::Info,
            description: "Response hedges despite strong belief evidence".into(),
            suggestion: "Beliefs support a more confident answer".into(),
        });
    }
}

/// Build a modified response that addresses critical issues.
fn build_modified_response(original: &str, issues: &[VerificationIssue]) -> String {
    let critical: Vec<&VerificationIssue> = issues.iter()
        .filter(|i| i.severity == IssueSeverity::Critical)
        .collect();

    let mut modified = original.to_string();
    if !critical.is_empty() {
        modified.push_str("\n\n---\nVerification notes:");
        for issue in &critical {
            modified.push_str(&format!("\n- {}: {}", issue.description, issue.suggestion));
        }
    }
    modified
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

/// Format verification issues for prompt injection.
pub fn format_for_prompt(result: &VerificationResult) -> Option<String> {
    if result.issues.is_empty() {
        return None;
    }
    let mut section = "# Verification Notes\n".to_string();
    for issue in &result.issues {
        let severity = match issue.severity {
            IssueSeverity::Critical => "CRITICAL",
            IssueSeverity::Warning => "WARNING",
            IssueSeverity::Info => "NOTE",
        };
        section.push_str(&format!("  [{}] {}\n", severity, issue.description));
    }
    Some(section)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(topic: &str, content: &str, conf: f64) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: content.into(), confidence: conf }
    }

    #[test]
    fn test_clean_response() {
        let beliefs = vec![b("rust", "Use Result for errors", 0.9)];
        let result = verify_response("Use Result<T,E> for recoverable errors", &beliefs, &[]);
        assert!(result.verified);
    }

    #[test]
    fn test_overconfidence_detection() {
        let beliefs = vec![b("uncertain", "Maybe this approach works", 0.5)];
        let result = verify_response(
            "You should definitely always use this approach",
            &beliefs, &[],
        );
        assert!(result.issues.iter().any(|i| i.check_type == VerificationCheck::OverConfidence));
    }

    #[test]
    fn test_historical_rejection() {
        let history = vec![
            ("no, that's wrong about the database migration approach using postgres".into(), "ok".into()),
        ];
        let result = verify_response(
            "You should use the database migration approach with postgres for this",
            &[], &history,
        );
        assert!(result.issues.iter().any(|i| i.check_type == VerificationCheck::HistoricalRejection));
    }

    #[test]
    fn test_empty_beliefs() {
        let result = verify_response("hello world", &[], &[]);
        assert!(result.verified);
        assert!(result.issues.is_empty());
    }
}
