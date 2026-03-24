//! DOM Parser — converts raw HTML + GetElements JSON into a semantic element tree.
//! Uses ARIA labels, HTML5 roles, structural proximity, and universal patterns.
//! No vision. No coordinates. No site-specific engineering.

use std::collections::HashMap;
use crate::types::{ElementRole, SemanticElement};

/// Parse a page into semantic elements from GetElements JSON + raw HTML.
pub fn parse_page(elements_json: &str, html: &str) -> Vec<SemanticElement> {
    let mut elements = parse_elements_json(elements_json);
    enrich_from_html(&mut elements, html);
    elements.retain(|e| e.role.is_actionable() || !e.label.is_empty());
    elements
}

/// Check if the DOM is parseable (vs canvas/SVG only).
pub fn is_dom_parseable(html: &str) -> bool {
    let lower = html.to_lowercase();
    // Canvas-only or SVG-only pages can't be semantically navigated
    if lower.contains("<canvas") && !lower.contains("<input") && !lower.contains("<button") {
        let text = strip_tags(html);
        if text.len() < 100 { return false; }
    }
    // Must have some interactive elements
    let interactive = ["<input", "<button", "<a ", "<select", "<textarea", "role=\"button\"", "role=\"link\""];
    interactive.iter().any(|tag| lower.contains(tag))
}

/// Parse the GetElements JSON array into SemanticElements.
fn parse_elements_json(json: &str) -> Vec<SemanticElement> {
    let items: Vec<serde_json::Value> = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    items.iter().filter_map(|item| {
        let tag = item.get("tag")?.as_str()?.to_string();
        let input_type = item.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());
        let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let name = item.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let id = item.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let href = item.get("href").and_then(|v| v.as_str()).map(|s| s.to_string());
        let placeholder = item.get("placeholder").and_then(|v| v.as_str()).map(|s| s.to_string());

        let role = ElementRole::infer(&tag, input_type.as_deref(), None);
        let label = derive_label(&text, &name, &placeholder, &id);
        let selector = derive_selector(&tag, &id, &name, &label, &input_type);

        Some(SemanticElement {
            selector, tag, role, label,
            input_type, href,
            is_visible: true, // GetElements only returns visible elements
            is_disabled: false,
            parent_context: None,
            aria: HashMap::new(),
        })
    }).collect()
}

/// Enrich elements with ARIA data, roles, landmarks, disabled state from HTML.
fn enrich_from_html(elements: &mut [SemanticElement], html: &str) {
    let lower = html.to_lowercase();
    for el in elements.iter_mut() {
        // Find the element in HTML by its selector components
        let search_key = if let Some(id) = el.selector.strip_prefix('#') {
            format!("id=\"{id}\"")
        } else if let Some(name) = el.selector.strip_prefix("[name=\"").and_then(|s| s.strip_suffix("\"]")) {
            format!("name=\"{name}\"")
        } else {
            continue;
        };

        if let Some(pos) = lower.find(&search_key.to_lowercase()) {
            let chunk = &html[pos.saturating_sub(200)..html.len().min(pos + 500)];
            let lower_chunk = chunk.to_lowercase();

            // Extract ARIA attributes
            for attr in &["aria-label", "aria-describedby", "aria-labelledby", "aria-expanded",
                          "aria-pressed", "aria-current", "aria-hidden", "aria-disabled"] {
                if let Some(val) = extract_attr(&lower_chunk, attr) {
                    el.aria.insert(attr.to_string(), val.clone());
                    if *attr == "aria-label" && el.label.is_empty() {
                        el.label = val;
                    }
                }
            }

            // Extract role attribute
            if let Some(role_val) = extract_attr(&lower_chunk, "role") {
                el.role = ElementRole::infer(&el.tag, el.input_type.as_deref(), Some(&role_val));
            }

            // Detect disabled state
            if lower_chunk.contains("disabled") || lower_chunk.contains("aria-disabled=\"true\"") {
                el.is_disabled = true;
            }

            // Detect parent landmark context
            for landmark in &["<nav", "<main", "<aside", "<header", "<footer", "<form"] {
                let before = &html[..pos.saturating_sub(200)];
                if let Some(lm_pos) = before.to_lowercase().rfind(landmark) {
                    let lm_chunk = &before[lm_pos..];
                    if let Some(lm_label) = extract_attr(&lm_chunk.to_lowercase(), "aria-label") {
                        el.parent_context = Some(lm_label);
                        break;
                    }
                    el.parent_context = Some(landmark.trim_start_matches('<').to_string());
                    break;
                }
            }
        }
    }
}

/// Derive best label from available text sources.
fn derive_label(text: &str, name: &Option<String>, placeholder: &Option<String>, id: &Option<String>) -> String {
    if !text.is_empty() { return text.to_string(); }
    if let Some(p) = placeholder { if !p.is_empty() { return p.clone(); } }
    if let Some(n) = name { if !n.is_empty() { return humanize(n); } }
    if let Some(i) = id { if !i.is_empty() { return humanize(i); } }
    String::new()
}

/// Derive a stable CSS selector (prefers id > name > aria-label > tag).
fn derive_selector(tag: &str, id: &Option<String>, name: &Option<String>,
    label: &str, input_type: &Option<String>) -> String {
    if let Some(id) = id { if !id.is_empty() { return format!("#{id}"); } }
    if let Some(name) = name { if !name.is_empty() { return format!("[name=\"{name}\"]"); } }
    if !label.is_empty() && label.len() < 50 {
        return format!("{tag}[aria-label=\"{label}\"]");
    }
    if let Some(t) = input_type {
        return format!("{tag}[type=\"{t}\"]");
    }
    tag.to_string()
}

/// Extract attribute value from HTML chunk.
fn extract_attr(html: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    let pos = html.find(&pattern)?;
    let start = pos + pattern.len();
    let end = html[start..].find('"')? + start;
    let val = html[start..end].trim().to_string();
    if val.is_empty() { None } else { Some(val) }
}

/// Convert camelCase/snake_case/kebab-case to human-readable.
fn humanize(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c == '_' || c == '-' { result.push(' '); }
        else if c.is_uppercase() && i > 0 { result.push(' '); result.push(c.to_lowercase().next().unwrap()); }
        else { result.push(c); }
    }
    result.trim().to_string()
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' { in_tag = true; } else if c == '>' { in_tag = false; }
        else if !in_tag { out.push(c); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_elements_json_basic() {
        let json = r#"[
            {"tag":"button","type":null,"text":"Submit","name":null,"id":"submit-btn","href":null,"placeholder":null},
            {"tag":"input","type":"email","text":"","name":"email","id":null,"href":null,"placeholder":"Enter email"}
        ]"#;
        let elements = parse_elements_json(json);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].role, ElementRole::Button);
        assert_eq!(elements[0].label, "Submit");
        assert_eq!(elements[0].selector, "#submit-btn");
        assert_eq!(elements[1].role, ElementRole::EmailInput);
        assert_eq!(elements[1].label, "Enter email");
        assert_eq!(elements[1].selector, "[name=\"email\"]");
    }

    #[test]
    fn is_parseable_normal_page() {
        let html = "<html><body><input type='text'><button>Click</button></body></html>";
        assert!(is_dom_parseable(html));
    }

    #[test]
    fn is_not_parseable_canvas_only() {
        let html = "<html><body><canvas id='game' width='800' height='600'></canvas></body></html>";
        assert!(!is_dom_parseable(html));
    }

    #[test]
    fn humanize_camel_case() {
        assert_eq!(humanize("submitButton"), "submit button");
        assert_eq!(humanize("user_email"), "user email");
        assert_eq!(humanize("search-query"), "search query");
    }

    #[test]
    fn derive_selector_prefers_id() {
        let sel = derive_selector("button", &Some("submit-btn".into()), &Some("submit".into()), "Submit", &None);
        assert_eq!(sel, "#submit-btn");
    }

    #[test]
    fn derive_selector_falls_to_name() {
        let sel = derive_selector("input", &None, &Some("email".into()), "", &Some("email".into()));
        assert_eq!(sel, "[name=\"email\"]");
    }

    #[test]
    fn extract_aria_label() {
        let html = r#"<button aria-label="Close dialog" class="x">X</button>"#;
        assert_eq!(extract_attr(html, "aria-label"), Some("Close dialog".into()));
    }

    #[test]
    fn enrich_adds_aria() {
        let json = r#"[{"tag":"button","type":null,"text":"X","name":null,"id":"close-btn","href":null,"placeholder":null}]"#;
        let html = r#"<button id="close-btn" aria-label="Close dialog">X</button>"#;
        let mut elements = parse_elements_json(json);
        enrich_from_html(&mut elements, html);
        assert_eq!(elements[0].aria.get("aria-label"), Some(&"close dialog".to_string()));
    }
}
