//! Self-improvement decision engine — analyzes conversations to suggest improvements.

pub struct ImprovementSuggestion {
    pub description: String,
    pub category: ImprovementCategory,
    pub impact: Impact,
    pub target_area: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Impact { High, Medium, Low }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImprovementCategory { BugFix, Feature, Performance, Ux }

/// Analyze conversation history for improvement opportunities (offline, no LLM).
pub fn analyze_conversation(history: &[(String, String)]) -> Vec<ImprovementSuggestion> {
    let mut suggestions = Vec::new();
    for (user_msg, hydra_resp) in history {
        let lower_resp = hydra_resp.to_lowercase();
        let lower_user = user_msg.to_lowercase();
        // Detect error responses
        if lower_resp.contains("error") || lower_resp.contains("failed") {
            suggestions.push(ImprovementSuggestion {
                description: format!("Error in response to: {}", truncate(user_msg, 60)),
                category: ImprovementCategory::BugFix,
                impact: Impact::High,
                target_area: "error_handling".into(),
            });
        }
        // Detect generic/unhelpful responses
        if lower_resp.contains("i can help") && hydra_resp.len() < 80 {
            suggestions.push(ImprovementSuggestion {
                description: "Response was too generic — needs more specificity".into(),
                category: ImprovementCategory::Ux,
                impact: Impact::Medium,
                target_area: "conversation".into(),
            });
        }
        // Detect user frustration
        if lower_user.contains("not working") || lower_user.contains("wrong")
            || lower_user.contains("still the same") {
            suggestions.push(ImprovementSuggestion {
                description: format!("User expressed frustration: {}", truncate(user_msg, 60)),
                category: ImprovementCategory::BugFix,
                impact: Impact::High,
                target_area: "reliability".into(),
            });
        }
    }
    suggestions
}

/// Extract error messages from conversation history.
pub fn detect_errors(history: &[(String, String)]) -> Vec<String> {
    let error_patterns = ["error", "failed", "cannot", "timed out", "builder error"];
    history.iter()
        .flat_map(|(user, hydra)| {
            let mut errs = Vec::new();
            for pat in &error_patterns {
                if hydra.to_lowercase().contains(pat) {
                    errs.push(hydra.clone());
                    break;
                }
                if user.to_lowercase().contains(pat) {
                    errs.push(user.clone());
                    break;
                }
            }
            errs
        })
        .collect()
}

/// Sort suggestions by impact (High first).
pub fn rank_suggestions(suggestions: &mut Vec<ImprovementSuggestion>) {
    suggestions.sort_by_key(|s| s.impact);
}

/// Convert a suggestion to a markdown spec format.
pub fn format_as_spec(suggestion: &ImprovementSuggestion) -> String {
    format!(
        "# SPEC: {desc}\n\n## Requirement\n{desc}\n\n## Acceptance Criteria\n\
         1. Fix identified issue\n2. Add test to prevent regression\n\n\
         ## Implementation Location\n- Target area: {area}\n- Category: {cat:?}\n- Impact: {imp:?}\n",
        desc = suggestion.description,
        area = suggestion.target_area,
        cat = suggestion.category,
        imp = suggestion.impact,
    )
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_finds_errors() {
        let history = vec![
            ("fix the build".into(), "Error: HTTP error: builder error".into()),
        ];
        let suggestions = analyze_conversation(&history);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].impact, Impact::High);
    }

    #[test]
    fn test_detect_errors() {
        let history = vec![
            ("hello".into(), "hi there!".into()),
            ("run it".into(), "failed to connect".into()),
        ];
        let errors = detect_errors(&history);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("failed"));
    }

    #[test]
    fn test_rank_suggestions() {
        let mut suggestions = vec![
            ImprovementSuggestion { description: "low".into(), category: ImprovementCategory::Ux, impact: Impact::Low, target_area: "ui".into() },
            ImprovementSuggestion { description: "high".into(), category: ImprovementCategory::BugFix, impact: Impact::High, target_area: "core".into() },
        ];
        rank_suggestions(&mut suggestions);
        assert_eq!(suggestions[0].impact, Impact::High);
    }

    #[test]
    fn test_format_as_spec() {
        let s = ImprovementSuggestion {
            description: "Fix timeout handling".into(),
            category: ImprovementCategory::BugFix,
            impact: Impact::High,
            target_area: "error_handling".into(),
        };
        let spec = format_as_spec(&s);
        assert!(spec.contains("# SPEC:"));
        assert!(spec.contains("Fix timeout handling"));
    }
}
