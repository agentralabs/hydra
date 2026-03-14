//! Coherence checking — validates response consistency with context.
//!
//! UCU Module #8 (Wave 3). Catches off-topic responses, contradictions with
//! memory/history, and empty substance. Lightweight complement to verify_response.rs
//! (which does full Veritas-powered claim verification).
//! Why not a sister? Quick coherence is pure string analysis — no I/O.

/// Result of a coherence check.
#[derive(Debug, Clone)]
pub struct CoherenceResult {
    pub coherent: bool,
    pub score: f32,
    pub issues: Vec<CoherenceIssue>,
}

/// A specific coherence issue found.
#[derive(Debug, Clone)]
pub struct CoherenceIssue {
    pub kind: CoherenceIssueKind,
    pub description: String,
    pub severity: f32,
}

/// Categories of coherence issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoherenceIssueKind {
    /// Response doesn't address the user's question.
    OffTopic,
    /// Contradicts something said earlier in conversation.
    ContradictsPrior,
    /// Contradicts stored memory/beliefs.
    ContradictsMemory,
    /// Doesn't answer all parts of a multi-part question.
    IncompleteAnswer,
    /// Response in wrong language.
    WrongLanguage,
    /// Filler text with no actionable content.
    EmptySubstance,
}

/// Quick coherence score (0.0–1.0) without history/memory checks.
/// Fast enough to run on every response.
pub fn quick_coherence(response: &str, user_input: &str) -> f32 {
    let mut score: f32 = 1.0;
    let response_lower = response.to_lowercase();
    let input_lower = user_input.to_lowercase();

    // Check for empty/near-empty response
    let trimmed = response.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    if trimmed.len() < 10 {
        score -= 0.4;
    }

    // Check topic overlap — do response and input share key words?
    let input_words: Vec<&str> = input_lower.split_whitespace()
        .filter(|w| w.len() >= 4)
        .collect();
    if !input_words.is_empty() {
        let overlap = input_words.iter()
            .filter(|w| response_lower.contains(*w))
            .count();
        let overlap_ratio = overlap as f32 / input_words.len() as f32;
        if overlap_ratio < 0.1 && response.len() > 100 {
            // Long response with almost no overlap with input — likely off-topic
            score -= 0.3;
        }
    }

    // Check for pure filler
    let filler_patterns = [
        "i understand your", "that's a great question",
        "let me think about", "absolutely, i can",
        "i'd be happy to", "sure, i can help",
    ];
    let filler_count = filler_patterns.iter()
        .filter(|p| response_lower.contains(*p))
        .count();
    if filler_count >= 2 && response.len() < 200 {
        score -= 0.2; // Mostly filler, little substance
    }

    // Check for wrong language (simple heuristic)
    let input_ascii_ratio = input_lower.chars().filter(|c| c.is_ascii_alphabetic()).count() as f32
        / (input_lower.len().max(1) as f32);
    let response_ascii_ratio = response_lower.chars().filter(|c| c.is_ascii_alphabetic()).count() as f32
        / (response_lower.len().max(1) as f32);
    if (input_ascii_ratio - response_ascii_ratio).abs() > 0.5 {
        score -= 0.2;
    }

    score.max(0.0).min(1.0)
}

/// Full coherence check including history and memory context.
pub fn check_coherence(
    response: &str,
    user_input: &str,
    history: &[(String, String)],
    memory_context: Option<&str>,
) -> CoherenceResult {
    let mut issues = Vec::new();
    let mut score = quick_coherence(response, user_input);

    // Check against conversation history for contradictions
    if let Some(contradiction) = check_history_contradiction(response, history) {
        score -= 0.3;
        issues.push(CoherenceIssue {
            kind: CoherenceIssueKind::ContradictsPrior,
            description: contradiction,
            severity: 0.7,
        });
    }

    // Check against memory context
    if let Some(mem) = memory_context {
        if let Some(conflict) = check_memory_contradiction(response, mem) {
            score -= 0.2;
            issues.push(CoherenceIssue {
                kind: CoherenceIssueKind::ContradictsMemory,
                description: conflict,
                severity: 0.5,
            });
        }
    }

    // Check for incomplete multi-part answers
    if let Some(missing) = check_completeness(response, user_input) {
        score -= 0.15;
        issues.push(CoherenceIssue {
            kind: CoherenceIssueKind::IncompleteAnswer,
            description: missing,
            severity: 0.4,
        });
    }

    // Off-topic detection
    if score < 0.4 && issues.is_empty() {
        issues.push(CoherenceIssue {
            kind: CoherenceIssueKind::OffTopic,
            description: "Response may not address the user's request".into(),
            severity: 0.6,
        });
    }

    let score = score.max(0.0).min(1.0);
    CoherenceResult {
        coherent: score >= 0.5 && issues.iter().all(|i| i.severity < 0.7),
        score,
        issues,
    }
}

/// Check if response contradicts recent history.
fn check_history_contradiction(
    response: &str,
    history: &[(String, String)],
) -> Option<String> {
    let response_lower = response.to_lowercase();
    // Look for direct negation of recent Hydra statements
    for (role, content) in history.iter().rev().take(5) {
        if role.contains("Hydra") || role.contains("hydra") || role.contains("assistant") {
            let content_lower = content.to_lowercase();
            // Check for "X is Y" in history vs "X is not Y" in response (or vice versa)
            if contains_negation_of(&response_lower, &content_lower) {
                return Some(format!("May contradict recent statement: '{}'",
                    &content[..content.len().min(80)]));
            }
        }
    }
    None
}

/// Simple negation check — detects "is not" vs "is" patterns.
fn contains_negation_of(a: &str, b: &str) -> bool {
    // Extract key claims from b (statements with "is", "are", "can", "will")
    for sentence in b.split('.') {
        let sentence = sentence.trim();
        if sentence.len() < 10 { continue; }
        // If b says "X is Y" and a says "X is not Y"
        if let Some(pos) = sentence.find(" is ") {
            let subject = &sentence[..pos];
            if subject.len() >= 3 && a.contains(subject) && a.contains(" is not ") {
                return true;
            }
        }
    }
    false
}

/// Check if response contradicts stored memory.
fn check_memory_contradiction(response: &str, memory: &str) -> Option<String> {
    // If memory says "[correction] X" and response does not-X
    for line in memory.lines() {
        if line.contains("[correction]") {
            let correction = line.replace("[correction]", "").trim().to_string();
            if !correction.is_empty() {
                let resp_lower = response.to_lowercase();
                let corr_lower = correction.to_lowercase();
                // Very basic: if correction mentions a pattern and response contradicts
                if corr_lower.contains("don't") || corr_lower.contains("never") {
                    let key_phrase = corr_lower.replace("don't ", "").replace("never ", "");
                    if resp_lower.contains(key_phrase.trim()) {
                        return Some(format!("May violate correction: {}", &correction[..correction.len().min(80)]));
                    }
                }
            }
        }
    }
    None
}

/// Check if a multi-part question was fully addressed.
fn check_completeness(response: &str, input: &str) -> Option<String> {
    let input_lower = input.to_lowercase();
    let response_lower = response.to_lowercase();

    // Count question marks — multiple questions expected multiple answers
    let question_count = input.chars().filter(|c| *c == '?').count();
    if question_count >= 2 {
        // Very rough: response should be proportionally longer for multi-questions
        if response.len() < question_count * 50 {
            return Some(format!("{} questions asked but response seems incomplete", question_count));
        }
    }

    // Numbered list in input
    if input.contains("1.") && input.contains("2.") {
        let has_1 = response.contains("1") || response_lower.contains("first");
        let has_2 = response.contains("2") || response_lower.contains("second");
        if !has_1 || !has_2 {
            return Some("Numbered items in request may not all be addressed".into());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_coherence_good() {
        let score = quick_coherence(
            "The Rust compiler uses LLVM for code generation and optimization.",
            "How does the Rust compiler work?",
        );
        assert!(score >= 0.7);
    }

    #[test]
    fn test_quick_coherence_empty() {
        assert_eq!(quick_coherence("", "test"), 0.0);
    }

    #[test]
    fn test_quick_coherence_filler() {
        let score = quick_coherence(
            "I understand your question. That's a great question. I'd be happy to help.",
            "How does X work?",
        );
        assert!(score < 0.9);
    }

    #[test]
    fn test_full_coherence_ok() {
        let result = check_coherence(
            "Rust uses ownership and borrowing for memory safety.",
            "Tell me about Rust memory management",
            &[],
            None,
        );
        assert!(result.coherent);
        assert!(result.score >= 0.7);
    }

    #[test]
    fn test_multi_question_incomplete() {
        let result = check_coherence(
            "Yes.",
            "1. What is Rust? 2. How does it compare to Go? 3. Which should I use?",
            &[],
            None,
        );
        assert!(result.issues.iter().any(|i| i.kind == CoherenceIssueKind::IncompleteAnswer));
    }
}
