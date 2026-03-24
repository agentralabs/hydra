//! Intent router — maps user goals to execution plans via IDF-weighted scoring.
//! Zero LLM tokens. Same mathematics proven in hydra-genome at 9.5/10.

use crate::types::*;

/// Minimum confidence to proceed with semantic nav (below = vision fallback).
const CONFIDENCE_THRESHOLD: f64 = 0.3;

/// Route a user goal to an execution plan using the page constitution.
pub fn route(goal: &str, constitution: &PageConstitution) -> Option<ExecutionPlan> {
    let goal_terms = stem_terms(goal);
    if goal_terms.is_empty() { return None; }

    // Score each element against the goal
    let mut scored: Vec<(&SemanticElement, f64)> = constitution.elements.iter()
        .filter(|e| e.is_visible && !e.is_disabled && e.role.is_actionable())
        .map(|e| {
            let el_terms = stem_terms(&e.search_text());
            let score = idf_score(&goal_terms, &el_terms);
            (e, score)
        })
        .filter(|(_, s)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Check if any form matches the intent
    if let Some(plan) = try_form_route(goal, &goal_terms, constitution) {
        return Some(plan);
    }

    // Check if search intent
    if let Some(plan) = try_search_route(goal, &goal_terms, constitution) {
        return Some(plan);
    }

    // Check if navigation intent
    if let Some(plan) = try_nav_route(&goal_terms, constitution) {
        return Some(plan);
    }

    // Check if click intent on best-scoring element
    if let Some((element, score)) = scored.first() {
        if *score >= CONFIDENCE_THRESHOLD {
            return Some(build_click_plan(element, *score, goal));
        }
    }

    None // Below threshold — fall back to vision
}

fn try_form_route(goal: &str, goal_terms: &[String], c: &PageConstitution) -> Option<ExecutionPlan> {
    for form in &c.forms {
        // Score form against goal
        let form_terms = stem_terms(&format!("{} {}", form.name, format!("{:?}", form.form_type)));
        let form_score = idf_score(goal_terms, &form_terms);

        // Also check if goal mentions any field labels
        let field_match = form.fields.iter().any(|f| {
            let ft = stem_terms(&f.search_text());
            idf_score(goal_terms, &ft) > 0.2
        });

        if form_score > 0.2 || field_match {
            let mut steps = Vec::new();
            // Extract content to type from goal (text after intent keywords)
            let content = extract_content_from_goal(goal);

            // Fill each field
            for field in &form.fields {
                let value = match_field_value(field, goal, &content);
                if !value.is_empty() {
                    steps.push(PlannedStep {
                        action: hydra_browser::BrowserAction::Click { selector: field.selector.clone() },
                        description: format!("Focus {}", field.label),
                        selector: field.selector.clone(),
                    });
                    steps.push(PlannedStep {
                        action: hydra_browser::BrowserAction::Type { selector: field.selector.clone(), text: value.clone() },
                        description: format!("Type '{}' into {}", value, field.label),
                        selector: field.selector.clone(),
                    });
                }
            }

            // Submit
            if let Some(submit) = &form.submit_selector {
                steps.push(PlannedStep {
                    action: hydra_browser::BrowserAction::Click { selector: submit.clone() },
                    description: "Submit form".into(),
                    selector: submit.clone(),
                });
            }

            if !steps.is_empty() {
                return Some(ExecutionPlan {
                    steps,
                    confidence: (form_score + 0.3).min(1.0),
                    strategy: format!("Fill {} form", form.name),
                });
            }
        }
    }
    None
}

fn try_search_route(goal: &str, goal_terms: &[String], c: &PageConstitution) -> Option<ExecutionPlan> {
    let search_sel = c.search_input.as_ref()?;
    let is_search_intent = goal_terms.iter().any(|t| t == "search" || t == "find" || t == "look");
    if !is_search_intent { return None; }

    let query = extract_content_from_goal(goal);
    if query.is_empty() { return None; }

    Some(ExecutionPlan {
        steps: vec![
            PlannedStep {
                action: hydra_browser::BrowserAction::Click { selector: search_sel.clone() },
                description: "Focus search input".into(), selector: search_sel.clone(),
            },
            PlannedStep {
                action: hydra_browser::BrowserAction::Type { selector: search_sel.clone(), text: query },
                description: "Type search query".into(), selector: search_sel.clone(),
            },
        ],
        confidence: 0.8,
        strategy: "Search via search input".into(),
    })
}

fn try_nav_route(goal_terms: &[String], c: &PageConstitution) -> Option<ExecutionPlan> {
    let mut best: Option<(&NavLink, f64)> = None;
    for link in &c.navigation {
        let link_terms = stem_terms(&format!("{} {}", link.label, link.href));
        let score = idf_score(goal_terms, &link_terms);
        if score > CONFIDENCE_THRESHOLD && best.as_ref().map_or(true, |(_, s)| score > *s) {
            best = Some((link, score));
        }
    }
    let (link, score) = best?;
    Some(ExecutionPlan {
        steps: vec![PlannedStep {
            action: hydra_browser::BrowserAction::Click { selector: link.selector.clone() },
            description: format!("Navigate to {}", link.label),
            selector: link.selector.clone(),
        }],
        confidence: score,
        strategy: format!("Navigate to {}", link.label),
    })
}

fn build_click_plan(element: &SemanticElement, score: f64, goal: &str) -> ExecutionPlan {
    let mut steps = vec![PlannedStep {
        action: hydra_browser::BrowserAction::Click { selector: element.selector.clone() },
        description: format!("Click {}", element.label),
        selector: element.selector.clone(),
    }];

    // If the element is a textarea/input, also type the content from goal
    if element.role.is_input() {
        let content = extract_content_from_goal(goal);
        if !content.is_empty() {
            steps.push(PlannedStep {
                action: hydra_browser::BrowserAction::Type { selector: element.selector.clone(), text: content },
                description: format!("Type into {}", element.label),
                selector: element.selector.clone(),
            });
        }
    }

    ExecutionPlan { steps, confidence: score, strategy: format!("Click {}", element.label) }
}

/// Extract the "content" from a goal (e.g., "post hello world" → "hello world").
fn extract_content_from_goal(goal: &str) -> String {
    let action_words = ["post", "type", "write", "send", "submit", "enter", "fill", "search",
        "find", "look", "click", "open", "go", "navigate", "visit", "on", "in", "into", "the",
        "a", "to", "for", "with"];
    let words: Vec<&str> = goal.split_whitespace()
        .filter(|w| !action_words.contains(&w.to_lowercase().as_str()))
        .filter(|w| !w.contains('.') || w.len() < 4) // skip domain names
        .collect();
    words.join(" ")
}

fn match_field_value(field: &SemanticElement, goal: &str, content: &str) -> String {
    // For compose/textarea, use the full extracted content
    if field.role == ElementRole::Textarea { return content.to_string(); }
    // For specific fields, try to match from goal
    let label_lower = field.label.to_lowercase();
    if label_lower.contains("email") {
        // Look for email pattern in goal
        for word in goal.split_whitespace() {
            if word.contains('@') { return word.to_string(); }
        }
    }
    // Default: use the content for the first field
    content.to_string()
}

// ── IDF Scoring (copied from hydra-genome, ~30 lines) ──

fn idf_score(query: &[String], doc: &[String]) -> f64 {
    if query.is_empty() || doc.is_empty() { return 0.0; }
    let mut matched_weight = 0.0;
    let mut total_weight = 0.0;
    for qt in query {
        let idf = 1.0 / (query.iter().filter(|t| *t == qt).count() as f64 + 0.5);
        total_weight += idf;
        if doc.iter().any(|dt| dt == qt || dt.contains(qt.as_str()) || qt.contains(dt.as_str())) {
            matched_weight += idf;
        }
    }
    if total_weight > 0.0 { matched_weight / total_weight } else { 0.0 }
}

fn stem_terms(text: &str) -> Vec<String> {
    let stop = ["the","a","an","is","are","was","do","does","how","what","why","when","where",
        "in","on","at","to","for","of","with","by","from","it","this","that","and","or","not"];
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2 && !stop.contains(w))
        .map(|w| stem(w))
        .collect()
}

fn stem(w: &str) -> String {
    for s in &["ship","ing","tion","ment","ness","able","ful","less","ous","ive","ly","er","ed","es","s"] {
        if w.len() > s.len() + 3 { if let Some(r) = w.strip_suffix(s) { return r.to_string(); } }
    }
    w.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idf_exact_match() {
        let q = stem_terms("post hello world");
        let d = stem_terms("create post");
        let score = idf_score(&q, &d);
        assert!(score > 0.2, "Expected match for 'post', got {score}");
    }

    #[test]
    fn idf_no_match() {
        let q = stem_terms("post hello");
        let d = stem_terms("settings profile");
        let score = idf_score(&q, &d);
        assert!(score < 0.1, "Expected no match, got {score}");
    }

    #[test]
    fn extract_content() {
        assert_eq!(extract_content_from_goal("post hello world"), "hello world");
        assert_eq!(extract_content_from_goal("search for rust ownership"), "rust ownership");
    }

    #[test]
    fn route_to_search() {
        let constitution = PageConstitution {
            url: "https://example.com".into(), title: "Test".into(),
            elements: vec![], forms: vec![], navigation: vec![],
            primary_action: None, search_input: Some("#search".into()),
            guards: vec![], parsed_at: chrono::Utc::now(),
        };
        let plan = route("search for rust ownership", &constitution);
        assert!(plan.is_some());
        assert!(plan.unwrap().strategy.contains("Search"));
    }
}
