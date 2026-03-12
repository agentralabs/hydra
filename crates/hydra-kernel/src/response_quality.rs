//! Response quality scorer — evaluates Hydra's responses using heuristics.

pub struct QualityScore {
    pub length_score: f32,
    pub error_free: bool,
    pub relevance: f32,
    pub specificity: f32,
    pub personality: f32,
    pub overall: f32,
}

/// Score a response against the question using heuristics.
pub fn score_response(question: &str, response: &str, user_name: &str) -> QualityScore {
    let length_score = calculate_length_score(question, response);
    let error_free = !has_error_indicators(response);
    let relevance = calculate_relevance(question, response);
    let specificity = calculate_specificity(response);
    let personality = calculate_personality(response, user_name);
    let error_penalty = if error_free { 0.0 } else { -0.2 };
    let overall = ((length_score + relevance + specificity + personality) / 4.0 + error_penalty)
        .clamp(0.0, 1.0);

    QualityScore { length_score, error_free, relevance, specificity, personality, overall }
}

/// Detect boilerplate/generic responses.
pub fn is_too_generic(response: &str) -> bool {
    let lower = response.to_lowercase();
    let generic = ["in conclusion", "as mentioned earlier", "it is important to note",
        "i hope this helps", "let me know if you need", "feel free to ask",
        "as an ai", "i don't have personal"];
    generic.iter().any(|p| lower.contains(p))
}

/// Suggest one improvement based on the lowest-scoring dimension.
pub fn suggest_improvement(score: &QualityScore) -> Option<String> {
    if score.length_score < 0.3 {
        return Some("Response length doesn't match question complexity.".into());
    }
    if !score.error_free {
        return Some("Response contains error indicators — check for failures.".into());
    }
    if score.relevance < 0.3 {
        return Some("Response doesn't address the question directly.".into());
    }
    if score.specificity < 0.3 {
        return Some("Response is too vague — add code, paths, or concrete suggestions.".into());
    }
    if score.personality < 0.3 {
        return Some("Response feels robotic — use the user's name or a warmer tone.".into());
    }
    None
}

fn has_error_indicators(response: &str) -> bool {
    let lower = response.to_lowercase();
    ["error:", "failed:", "cannot ", "http error", "builder error", "timed out"]
        .iter().any(|e| lower.contains(e))
}

fn calculate_length_score(question: &str, response: &str) -> f32 {
    let q_words = question.split_whitespace().count();
    let r_words = response.split_whitespace().count();
    // Short question → short answer is fine. Long question → expect longer answer.
    let expected_min = if q_words < 5 { 5 } else { q_words * 2 };
    let expected_max = expected_min * 10;
    if r_words < expected_min { r_words as f32 / expected_min as f32 }
    else if r_words > expected_max { (expected_max as f32) / (r_words as f32) }
    else { 1.0 }
}

fn calculate_relevance(question: &str, response: &str) -> f32 {
    let q_words: std::collections::HashSet<String> = question.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() >= 3)
        .collect();
    if q_words.is_empty() { return 0.5; }
    let r_lower = response.to_lowercase();
    let matches = q_words.iter().filter(|w| r_lower.contains(w.as_str())).count();
    (matches as f32 / q_words.len() as f32).clamp(0.0, 1.0)
}

fn calculate_specificity(response: &str) -> f32 {
    let mut score = 0.3_f32; // Base
    if response.contains("```") || response.contains("fn ") { score += 0.3; }
    if response.contains("crates/") || response.contains(".rs") || response.contains(".ts") { score += 0.2; }
    if response.chars().any(|c| c.is_ascii_digit()) { score += 0.1; }
    if response.contains('/') && response.contains('.') { score += 0.1; }
    score.clamp(0.0, 1.0)
}

fn calculate_personality(response: &str, user_name: &str) -> f32 {
    let mut score = 0.3_f32;
    if !user_name.is_empty() && response.contains(user_name) { score += 0.3; }
    let warm = ["!", "great", "sure", "happy to", "glad", "hey", "nice"];
    if warm.iter().any(|w| response.to_lowercase().contains(w)) { score += 0.2; }
    if !is_too_generic(response) { score += 0.2; }
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_good_response() {
        let score = score_response(
            "What is Rust?",
            "Rust is a systems programming language focused on safety and performance. It prevents memory bugs at compile time using its ownership system.",
            "mor",
        );
        assert!(score.overall > 0.4);
        assert!(score.error_free);
    }

    #[test]
    fn test_error_response() {
        let score = score_response(
            "fix the build",
            "Error: HTTP error: builder error",
            "mor",
        );
        assert!(!score.error_free);
        assert!(score.overall < 0.5);
    }

    #[test]
    fn test_generic_detection() {
        assert!(is_too_generic("In conclusion, this is a good approach."));
        assert!(!is_too_generic("The fix is in crates/hydra-kernel/src/lib.rs line 42."));
    }

    #[test]
    fn test_specific_response_scores_higher() {
        let vague = score_response("fix the bug", "I can help with that.", "user");
        let specific = score_response("fix the bug", "The bug is in crates/hydra-kernel/src/lib.rs — the fn parse() is missing a match arm.", "user");
        assert!(specific.specificity > vague.specificity);
    }

    #[test]
    fn test_suggest_improvement() {
        let bad = QualityScore { length_score: 0.2, error_free: true, relevance: 0.8, specificity: 0.8, personality: 0.8, overall: 0.6 };
        assert!(suggest_improvement(&bad).is_some());
        let good = QualityScore { length_score: 0.8, error_free: true, relevance: 0.8, specificity: 0.8, personality: 0.8, overall: 0.8 };
        assert!(suggest_improvement(&good).is_none());
    }
}
