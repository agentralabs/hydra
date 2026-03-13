//! Adaptive user model — learns coding style, expertise, and preferences
//! from interaction patterns. Persists to DB for cross-session continuity.

use std::collections::HashMap;

/// A learned trait about the user.
#[derive(Debug, Clone)]
pub struct LearnedTrait {
    pub value: String,
    pub confidence: f64,
    pub observations: u32,
}

/// Adaptive user model built from interaction patterns.
#[derive(Debug)]
pub struct UserModel {
    traits: HashMap<String, LearnedTrait>,
}

impl Default for UserModel {
    fn default() -> Self {
        Self::new()
    }
}

impl UserModel {
    pub fn new() -> Self {
        Self { traits: HashMap::new() }
    }

    /// Load traits from DB rows.
    pub fn load_from_db(&mut self, rows: &[(String, String, f64, i64)]) {
        for (key, value, confidence, count) in rows {
            self.traits.insert(key.clone(), LearnedTrait {
                value: value.clone(),
                confidence: *confidence,
                observations: *count as u32,
            });
        }
    }

    /// Observe an interaction and update traits.
    pub fn observe_interaction(
        &mut self,
        text: &str,
        _response: &str,
        success: bool,
    ) {
        let lower = text.to_lowercase();

        // Detect expertise level from vocabulary
        self.detect_expertise(&lower);

        // Detect verbosity preferences from corrections
        self.detect_verbosity(&lower);

        // Track preferred language from file extensions
        self.detect_language(&lower);

        // Track success patterns
        if success {
            self.increment_trait("successful_interactions", "count", 0.5);
        }
    }

    /// Get a learned trait.
    pub fn get_trait(&self, key: &str) -> Option<&LearnedTrait> {
        self.traits.get(key)
    }

    /// Generate system prompt additions from learned traits.
    pub fn system_prompt_additions(&self) -> String {
        let mut additions = Vec::new();

        if let Some(t) = self.traits.get("expertise_level") {
            if t.confidence >= 0.6 {
                match t.value.as_str() {
                    "expert" => additions.push(
                        "The user is an experienced developer. Be concise, skip basics."
                    ),
                    "intermediate" => additions.push(
                        "The user has moderate experience. Explain non-obvious decisions."
                    ),
                    "beginner" => additions.push(
                        "The user is learning. Explain step by step with reasoning."
                    ),
                    _ => {}
                }
            }
        }

        if let Some(t) = self.traits.get("verbosity") {
            if t.confidence >= 0.5 {
                match t.value.as_str() {
                    "concise" => additions.push("Keep responses short and direct."),
                    "verbose" => additions.push("The user prefers detailed explanations."),
                    _ => {}
                }
            }
        }

        if let Some(t) = self.traits.get("preferred_language") {
            if t.confidence >= 0.6 {
                additions.push("Primary language context noted.");
            }
        }

        if additions.is_empty() {
            String::new()
        } else {
            format!("\n## User Profile\n{}", additions.join("\n"))
        }
    }

    /// Get all traits for DB persistence.
    pub fn traits_for_db(&self) -> Vec<(&str, &str, f64)> {
        self.traits.iter()
            .map(|(k, v)| (k.as_str(), v.value.as_str(), v.confidence))
            .collect()
    }

    fn detect_expertise(&mut self, text: &str) {
        let expert_words = [
            "monomorphization", "vtable", "lifetime", "borrow checker",
            "trait object", "async runtime", "macro_rules", "unsafe",
            "zero-cost abstraction", "pinning", "waker", "tokio::spawn",
        ];
        let beginner_words = [
            "how do i", "what is a", "what does", "explain",
            "i don't understand", "help me", "tutorial",
        ];

        let expert_hits: usize = expert_words.iter().filter(|w| text.contains(*w)).count();
        let beginner_hits: usize = beginner_words.iter().filter(|w| text.contains(*w)).count();

        if expert_hits >= 2 {
            self.update_trait("expertise_level", "expert", 0.3);
        } else if beginner_hits >= 2 {
            self.update_trait("expertise_level", "beginner", 0.3);
        } else if expert_hits == 1 {
            self.update_trait("expertise_level", "intermediate", 0.1);
        }
    }

    fn detect_verbosity(&mut self, text: &str) {
        if text.contains("too long") || text.contains("shorter") || text.contains("tldr")
            || text.contains("just the code") || text.contains("be brief")
        {
            self.update_trait("verbosity", "concise", 0.4);
        } else if text.contains("explain more") || text.contains("tell me more")
            || text.contains("in detail") || text.contains("elaborate")
        {
            self.update_trait("verbosity", "verbose", 0.4);
        }
    }

    fn detect_language(&mut self, text: &str) {
        let langs = [
            (".rs", "rust"), (".ts", "typescript"), (".py", "python"),
            (".go", "go"), (".js", "javascript"), (".java", "java"),
        ];
        for (ext, lang) in &langs {
            if text.contains(ext) {
                self.update_trait("preferred_language", lang, 0.15);
                break;
            }
        }
    }

    fn update_trait(&mut self, key: &str, value: &str, confidence_delta: f64) {
        let entry = self.traits.entry(key.to_string()).or_insert_with(|| LearnedTrait {
            value: value.to_string(),
            confidence: 0.0,
            observations: 0,
        });

        if entry.value == value {
            entry.confidence = (entry.confidence + confidence_delta).min(1.0);
        } else if confidence_delta > entry.confidence {
            // New value has stronger signal — replace
            entry.value = value.to_string();
            entry.confidence = confidence_delta;
        }
        entry.observations += 1;
    }

    fn increment_trait(&mut self, key: &str, value: &str, confidence: f64) {
        let entry = self.traits.entry(key.to_string()).or_insert_with(|| LearnedTrait {
            value: value.to_string(),
            confidence,
            observations: 0,
        });
        entry.observations += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_expert() {
        let mut model = UserModel::new();
        model.observe_interaction(
            "how does the borrow checker handle lifetime elision with trait objects?",
            "response", true,
        );
        let t = model.get_trait("expertise_level").unwrap();
        assert_eq!(t.value, "expert");
    }

    #[test]
    fn test_detect_beginner() {
        let mut model = UserModel::new();
        model.observe_interaction(
            "what is a variable? how do i make one? i don't understand",
            "response", true,
        );
        let t = model.get_trait("expertise_level").unwrap();
        assert_eq!(t.value, "beginner");
    }

    #[test]
    fn test_detect_concise() {
        let mut model = UserModel::new();
        model.observe_interaction("too long, just the code", "response", true);
        let t = model.get_trait("verbosity").unwrap();
        assert_eq!(t.value, "concise");
    }

    #[test]
    fn test_detect_language() {
        let mut model = UserModel::new();
        model.observe_interaction("fix the bug in auth.rs", "response", true);
        let t = model.get_trait("preferred_language").unwrap();
        assert_eq!(t.value, "rust");
    }

    #[test]
    fn test_system_prompt_expert() {
        let mut model = UserModel::new();
        model.traits.insert("expertise_level".into(), LearnedTrait {
            value: "expert".into(), confidence: 0.8, observations: 10,
        });
        let prompt = model.system_prompt_additions();
        assert!(prompt.contains("concise"));
    }

    #[test]
    fn test_system_prompt_empty() {
        let model = UserModel::new();
        assert!(model.system_prompt_additions().is_empty());
    }

    #[test]
    fn test_load_from_db() {
        let mut model = UserModel::new();
        model.load_from_db(&[
            ("expertise_level".into(), "expert".into(), 0.9, 15),
            ("verbosity".into(), "concise".into(), 0.7, 5),
        ]);
        assert_eq!(model.get_trait("expertise_level").unwrap().value, "expert");
        assert_eq!(model.get_trait("verbosity").unwrap().observations, 5);
    }

    #[test]
    fn test_traits_for_db() {
        let mut model = UserModel::new();
        model.observe_interaction("fix auth.rs using the borrow checker lifetime rules", "ok", true);
        let db_traits = model.traits_for_db();
        assert!(!db_traits.is_empty());
    }

    #[test]
    fn test_confidence_accumulates() {
        let mut model = UserModel::new();
        model.observe_interaction("fix auth.rs", "ok", true);
        model.observe_interaction("update main.rs", "ok", true);
        let t = model.get_trait("preferred_language").unwrap();
        assert!(t.confidence > 0.15); // accumulated from 2 observations
        assert_eq!(t.observations, 2);
    }
}
