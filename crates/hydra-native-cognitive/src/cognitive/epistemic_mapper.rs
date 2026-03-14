//! Epistemic Mapper — maps what Hydra knows, doesn't know, and is uncertain
//! about for every query. Injects epistemic honesty into responses.
//!
//! Why isn't a sister doing this? Pure in-memory classification of already-loaded
//! beliefs. No I/O, no sister tools needed.

use hydra_native_state::operational_profile::ProfileBelief;

/// Epistemic map for a query — what we know, what's uncertain, what's missing.
#[derive(Debug, Clone)]
pub struct EpistemicMap {
    pub strong_ground: Vec<MappedBelief>,
    pub uncertain_ground: Vec<MappedBelief>,
    pub knowledge_gaps: Vec<String>,
    pub change_triggers: Vec<String>,
    pub overall_confidence: f64,
    pub recommendation_possible: bool,
}

/// A belief matched to a query with relevance info.
#[derive(Debug, Clone)]
pub struct MappedBelief {
    pub topic: String,
    pub content: String,
    pub confidence: f64,
}

/// Build an epistemic map for a query given the active beliefs.
pub fn map_knowledge(query: &str, beliefs: &[ProfileBelief]) -> EpistemicMap {
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace()
        .filter(|w| w.len() >= 3)
        .collect();

    if beliefs.is_empty() || query_words.is_empty() {
        return EpistemicMap {
            strong_ground: vec![],
            uncertain_ground: vec![],
            knowledge_gaps: vec!["No profile beliefs loaded".into()],
            change_triggers: vec![],
            overall_confidence: 0.0,
            recommendation_possible: false,
        };
    }

    // Match beliefs to query by keyword overlap
    let mut strong = Vec::new();
    let mut uncertain = Vec::new();

    for belief in beliefs {
        let belief_text = format!("{} {}", belief.topic, belief.content).to_lowercase();
        let relevance = query_words.iter()
            .filter(|w| belief_text.contains(*w))
            .count();

        if relevance == 0 {
            continue;
        }

        let mapped = MappedBelief {
            topic: belief.topic.clone(),
            content: belief.content.clone(),
            confidence: belief.confidence,
        };

        if belief.confidence >= 0.85 {
            strong.push(mapped);
        } else {
            uncertain.push(mapped);
        }
    }

    // Identify knowledge gaps — topics in query not covered by any belief
    let covered_topics: Vec<String> = strong.iter().chain(uncertain.iter())
        .map(|b| b.topic.to_lowercase())
        .collect();

    let gaps: Vec<String> = query_words.iter()
        .filter(|w| {
            !covered_topics.iter().any(|t| t.contains(*w))
                && !is_stop_word(w)
        })
        .map(|w| format!("No beliefs covering '{}'", w))
        .collect();

    // Change triggers — what would shift the assessment
    let mut triggers = Vec::new();
    for u in &uncertain {
        triggers.push(format!(
            "If confidence on '{}' rises above 85%, recommendation strengthens",
            truncate(&u.content, 60),
        ));
    }
    if !gaps.is_empty() {
        triggers.push("Filling knowledge gaps would improve assessment quality".into());
    }

    let total = strong.len() + uncertain.len();
    let overall = if total > 0 {
        let sum: f64 = strong.iter().chain(uncertain.iter())
            .map(|b| b.confidence).sum();
        sum / total as f64
    } else {
        0.0
    };

    let recommendation_possible = strong.len() >= 2 && overall >= 0.7;

    EpistemicMap {
        strong_ground: strong,
        uncertain_ground: uncertain,
        knowledge_gaps: gaps,
        change_triggers: triggers,
        overall_confidence: overall,
        recommendation_possible,
    }
}

/// Format epistemic map as a prompt section for LLM context injection.
pub fn format_for_prompt(map: &EpistemicMap) -> Option<String> {
    if map.strong_ground.is_empty() && map.uncertain_ground.is_empty() {
        return None;
    }

    let mut section = String::from("# Epistemic Map\n");

    if !map.strong_ground.is_empty() {
        section.push_str("STRONG GROUND (high confidence):\n");
        for b in &map.strong_ground {
            section.push_str(&format!(
                "  [{:.0}%] {}\n", b.confidence * 100.0, truncate(&b.content, 80),
            ));
        }
    }

    if !map.uncertain_ground.is_empty() {
        section.push_str("UNCERTAIN GROUND (moderate confidence):\n");
        for b in &map.uncertain_ground {
            section.push_str(&format!(
                "  [{:.0}%] {}\n", b.confidence * 100.0, truncate(&b.content, 80),
            ));
        }
    }

    if !map.knowledge_gaps.is_empty() {
        section.push_str("KNOWLEDGE GAPS:\n");
        for g in map.knowledge_gaps.iter().take(5) {
            section.push_str(&format!("  - {}\n", g));
        }
    }

    section.push_str(&format!(
        "Overall confidence: {:.0}% | Can recommend: {}\n",
        map.overall_confidence * 100.0,
        if map.recommendation_possible { "yes" } else { "insufficient ground" },
    ));
    section.push_str(
        "Be epistemically honest: state confidence levels, acknowledge gaps, \
         express uncertainty where it exists.\n"
    );

    Some(section)
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

fn is_stop_word(w: &str) -> bool {
    matches!(w, "the" | "and" | "for" | "with" | "this" | "that" | "from"
        | "should" | "would" | "could" | "what" | "how" | "why" | "when"
        | "where" | "which" | "about" | "does" | "can" | "use" | "using")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_belief(topic: &str, content: &str, confidence: f64) -> ProfileBelief {
        ProfileBelief { topic: topic.into(), content: content.into(), confidence }
    }

    #[test]
    fn test_empty_beliefs() {
        let map = map_knowledge("test query", &[]);
        assert!(map.strong_ground.is_empty());
        assert!(!map.recommendation_possible);
    }

    #[test]
    fn test_strong_vs_uncertain() {
        let beliefs = vec![
            make_belief("rust/ownership", "Ownership prevents data races", 0.95),
            make_belief("rust/async", "Tokio spawn for independent tasks", 0.70),
        ];
        let map = map_knowledge("rust ownership async", &beliefs);
        assert_eq!(map.strong_ground.len(), 1);
        assert_eq!(map.uncertain_ground.len(), 1);
    }

    #[test]
    fn test_format_prompt() {
        let beliefs = vec![
            make_belief("rust/ownership", "Ownership prevents data races", 0.95),
        ];
        let map = map_knowledge("rust ownership", &beliefs);
        let prompt = format_for_prompt(&map);
        assert!(prompt.is_some());
        assert!(prompt.unwrap().contains("STRONG GROUND"));
    }

    #[test]
    fn test_recommendation_threshold() {
        let beliefs = vec![
            make_belief("deploy/strategy", "Canary for high traffic", 0.90),
            make_belief("deploy/rollback", "Always have rollback plan", 0.92),
        ];
        let map = map_knowledge("deploy strategy rollback", &beliefs);
        assert!(map.recommendation_possible);
    }
}
