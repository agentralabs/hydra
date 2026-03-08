use hydra_core::types::{
    Action, ActionType, CompiledIntent, Goal, GoalType, IntentSource, VeritasValidation,
};
use uuid::Uuid;

/// Pattern for local classification
struct Pattern {
    keywords: Vec<&'static str>,
    goal_type: GoalType,
    action_type: ActionType,
}

/// Local intent classifier — Layer 2 of the 4-layer escalation (0 tokens)
/// Handles ~80% of common intents without any LLM call.
pub struct LocalClassifier {
    patterns: Vec<Pattern>,
}

impl LocalClassifier {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // File operations
                Pattern {
                    keywords: vec!["list", "show", "ls", "files", "dir"],
                    goal_type: GoalType::Query,
                    action_type: ActionType::Read,
                },
                Pattern {
                    keywords: vec!["create", "make", "touch", "new file"],
                    goal_type: GoalType::Create,
                    action_type: ActionType::FileCreate,
                },
                Pattern {
                    keywords: vec!["delete", "remove", "rm"],
                    goal_type: GoalType::Delete,
                    action_type: ActionType::FileDelete,
                },
                Pattern {
                    keywords: vec!["edit", "modify", "change", "update"],
                    goal_type: GoalType::Modify,
                    action_type: ActionType::FileModify,
                },
                Pattern {
                    keywords: vec!["read", "cat", "view", "open"],
                    goal_type: GoalType::Query,
                    action_type: ActionType::Read,
                },
                // Code operations
                Pattern {
                    keywords: vec!["run", "execute", "test"],
                    goal_type: GoalType::Execute,
                    action_type: ActionType::Execute,
                },
                Pattern {
                    keywords: vec!["build", "compile"],
                    goal_type: GoalType::Execute,
                    action_type: ActionType::Execute,
                },
                Pattern {
                    keywords: vec!["deploy"],
                    goal_type: GoalType::Deploy,
                    action_type: ActionType::Execute,
                },
                Pattern {
                    keywords: vec!["debug", "fix", "bug"],
                    goal_type: GoalType::Debug,
                    action_type: ActionType::Read,
                },
                Pattern {
                    keywords: vec!["refactor"],
                    goal_type: GoalType::Modify,
                    action_type: ActionType::FileModify,
                },
                // Git operations
                Pattern {
                    keywords: vec!["commit", "push", "pull", "merge", "git"],
                    goal_type: GoalType::Execute,
                    action_type: ActionType::GitOperation,
                },
                // Query operations
                Pattern {
                    keywords: vec!["explain", "what", "how", "why", "describe"],
                    goal_type: GoalType::Explain,
                    action_type: ActionType::Read,
                },
                Pattern {
                    keywords: vec!["search", "find", "grep", "look for"],
                    goal_type: GoalType::Query,
                    action_type: ActionType::Read,
                },
                // Review
                Pattern {
                    keywords: vec!["review", "check", "analyze", "audit"],
                    goal_type: GoalType::Review,
                    action_type: ActionType::Read,
                },
            ],
        }
    }

    /// Classify text using local patterns (0 tokens)
    pub fn classify(&self, text: &str) -> Option<CompiledIntent> {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        let mut best_match: Option<(&Pattern, usize)> = None;

        for pattern in &self.patterns {
            let match_count = pattern
                .keywords
                .iter()
                .filter(|kw| words.iter().any(|w| w.contains(*kw)))
                .count();

            if match_count > 0 && (best_match.is_none() || match_count > best_match.unwrap().1) {
                best_match = Some((pattern, match_count));
            }
        }

        best_match.map(|(pattern, match_count)| {
            let confidence = (match_count as f64 / pattern.keywords.len() as f64).min(0.95);
            CompiledIntent {
                id: Uuid::new_v4(),
                raw_text: text.to_string(),
                source: IntentSource::Cli,
                goal: Goal {
                    goal_type: pattern.goal_type.clone(),
                    target: extract_target(text),
                    outcome: text.to_string(),
                    sub_goals: vec![],
                },
                entities: vec![],
                actions: vec![Action::new(
                    pattern.action_type.clone(),
                    extract_target(text),
                )],
                constraints: vec![],
                success_criteria: vec![],
                confidence,
                estimated_steps: 1,
                tokens_used: 0, // Local — zero tokens!
                veritas_validation: VeritasValidation {
                    validated: false,
                    safety_score: 1.0,
                    warnings: vec![],
                },
            }
        })
    }
}

impl Default for LocalClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple target extraction — takes the last significant words
fn extract_target(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= 2 {
        text.to_string()
    } else {
        // Skip the verb (first word) and take the rest
        words[1..].join(" ")
    }
}
