//! NeedAnticipator — anticipate user needs based on behavioral patterns.

use serde::{Deserialize, Serialize};

/// Category of anticipated need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeedCategory {
    Information,
    Action,
    Reminder,
    Optimization,
    Safety,
}

/// An anticipated user need
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnticipatedNeed {
    pub id: String,
    pub category: NeedCategory,
    pub description: String,
    pub confidence: f64,
    pub suggested_action: String,
    pub timestamp: String,
}

/// Anticipates user needs from behavioral signals
pub struct NeedAnticipator {
    needs: parking_lot::RwLock<Vec<AnticipatedNeed>>,
    rules: Vec<AnticipationRule>,
}

struct AnticipationRule {
    trigger: String,
    category: NeedCategory,
    description: String,
    action: String,
    confidence: f64,
}

impl NeedAnticipator {
    pub fn new() -> Self {
        Self {
            needs: parking_lot::RwLock::new(Vec::new()),
            rules: default_rules(),
        }
    }

    /// Evaluate current context against anticipation rules
    pub fn evaluate(&self, context_keywords: &[&str]) -> Vec<AnticipatedNeed> {
        let mut needs = Vec::new();
        let context_str: Vec<String> = context_keywords.iter().map(|k| k.to_lowercase()).collect();

        for rule in &self.rules {
            if context_str.iter().any(|k| k.contains(&rule.trigger.to_lowercase())) {
                needs.push(AnticipatedNeed {
                    id: uuid::Uuid::new_v4().to_string(),
                    category: rule.category,
                    description: rule.description.clone(),
                    confidence: rule.confidence,
                    suggested_action: rule.action.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
        }

        self.needs.write().extend(needs.clone());
        needs
    }

    pub fn need_count(&self) -> usize {
        self.needs.read().len()
    }
}

impl Default for NeedAnticipator {
    fn default() -> Self {
        Self::new()
    }
}

fn default_rules() -> Vec<AnticipationRule> {
    vec![
        AnticipationRule {
            trigger: "error".into(),
            category: NeedCategory::Action,
            description: "Errors detected — may need debugging assistance".into(),
            action: "Offer to analyze error logs".into(),
            confidence: 0.7,
        },
        AnticipationRule {
            trigger: "deploy".into(),
            category: NeedCategory::Safety,
            description: "Deployment activity — may need pre-flight check".into(),
            action: "Run deployment checklist".into(),
            confidence: 0.8,
        },
        AnticipationRule {
            trigger: "test".into(),
            category: NeedCategory::Information,
            description: "Testing activity — may need test coverage report".into(),
            action: "Generate test coverage summary".into(),
            confidence: 0.6,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_need_anticipation() {
        let anticipator = NeedAnticipator::new();
        let needs = anticipator.evaluate(&["build error", "compile failed"]);
        assert!(!needs.is_empty());
        assert!(needs.iter().any(|n| n.category == NeedCategory::Action));
    }

    #[test]
    fn test_no_match() {
        let anticipator = NeedAnticipator::new();
        let needs = anticipator.evaluate(&["reading documentation"]);
        assert!(needs.is_empty());
    }

    #[test]
    fn test_multiple_matches() {
        let anticipator = NeedAnticipator::new();
        let needs = anticipator.evaluate(&["error in test"]);
        assert!(needs.len() >= 2); // matches "error" and "test"
    }
}
