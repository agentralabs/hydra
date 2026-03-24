//! Affordance engine — converts semantic elements into a PageConstitution.
//! Detects forms, navigation, CTA, search, and guards (cookie banners, modals).

use crate::types::*;

/// Build a page constitution from parsed semantic elements.
pub fn build_constitution(elements: Vec<SemanticElement>, html: &str, url: &str) -> PageConstitution {
    let title = extract_title(html);
    let forms = detect_forms(&elements, html);
    let navigation = detect_navigation(&elements);
    let primary_action = detect_primary_cta(&elements, &forms);
    let search_input = detect_search_input(&elements);
    let guards = detect_guards(&elements, html);

    PageConstitution {
        url: url.to_string(),
        title,
        elements,
        forms,
        navigation,
        primary_action,
        search_input,
        guards,
        parsed_at: chrono::Utc::now(),
    }
}

/// Group elements into form structures.
fn detect_forms(elements: &[SemanticElement], html: &str) -> Vec<SemanticForm> {
    let mut forms = Vec::new();
    let lower = html.to_lowercase();

    // Find form boundaries in HTML
    let mut form_ranges: Vec<(usize, usize, Option<String>)> = Vec::new();
    let mut pos = 0;
    while let Some(start) = lower[pos..].find("<form") {
        let abs = pos + start;
        if let Some(end) = lower[abs..].find("</form>") {
            let name = extract_form_name(&html[abs..abs + end]);
            form_ranges.push((abs, abs + end, name));
            pos = abs + end;
        } else { break; }
    }

    // Group elements by form context
    for (i, (start, end, name)) in form_ranges.iter().enumerate() {
        let form_html = &lower[*start..*end];
        let mut fields: Vec<SemanticElement> = Vec::new();
        let mut submit = None;

        for el in elements {
            // Check if element belongs to this form by selector match in form HTML
            let sel_lower = el.selector.to_lowercase();
            let in_form = form_html.contains(&sel_lower)
                || el.parent_context.as_ref().map_or(false, |ctx| ctx.contains("form"));

            if in_form || (el.parent_context.is_none() && form_ranges.len() == 1) {
                if el.role == ElementRole::Button && submit.is_none() {
                    submit = Some(el.selector.clone());
                }
                if el.role.is_input() {
                    fields.push(el.clone());
                }
            }
        }

        if !fields.is_empty() {
            let form_type = classify_form(&fields, form_html);
            forms.push(SemanticForm {
                name: name.clone().unwrap_or_else(|| format!("form_{i}")),
                fields,
                submit_selector: submit,
                form_type,
            });
        }
    }

    // If no <form> tags but there are inputs + a button, create an implicit form
    if forms.is_empty() {
        let inputs: Vec<_> = elements.iter().filter(|e| e.role.is_input()).cloned().collect();
        let submit = elements.iter().find(|e| e.role == ElementRole::Button).map(|e| e.selector.clone());
        if !inputs.is_empty() {
            let form_type = classify_form(&inputs, &lower);
            forms.push(SemanticForm {
                name: "implicit_form".into(),
                fields: inputs,
                submit_selector: submit,
                form_type,
            });
        }
    }

    forms
}

/// Classify a form's intent from its fields.
fn classify_form(fields: &[SemanticElement], html: &str) -> FormIntent {
    let has_password = fields.iter().any(|f| f.role == ElementRole::PasswordInput);
    let has_email = fields.iter().any(|f| f.role == ElementRole::EmailInput);
    let has_search = fields.iter().any(|f| f.role == ElementRole::SearchInput);
    let has_textarea = fields.iter().any(|f| f.role == ElementRole::Textarea);

    if has_search { return FormIntent::Search; }
    if has_password && fields.len() <= 3 { return FormIntent::Login; }
    if has_password && has_email && fields.len() > 3 { return FormIntent::Registration; }
    if has_textarea { return FormIntent::Compose; }
    if html.contains("login") || html.contains("sign in") || html.contains("signin") {
        return FormIntent::Login;
    }
    FormIntent::DataEntry
}

/// Detect navigation links (inside <nav> or <header>).
fn detect_navigation(elements: &[SemanticElement]) -> Vec<NavLink> {
    elements.iter()
        .filter(|e| {
            e.role == ElementRole::Link
                && e.href.is_some()
                && (e.parent_context.as_deref() == Some("nav")
                    || e.parent_context.as_deref() == Some("header"))
        })
        .map(|e| NavLink {
            selector: e.selector.clone(),
            label: e.label.clone(),
            href: e.href.clone().unwrap_or_default(),
            is_current: e.aria.get("aria-current").map_or(false, |v| v == "page"),
        })
        .collect()
}

/// Detect the primary call-to-action button.
fn detect_primary_cta(elements: &[SemanticElement], forms: &[SemanticForm]) -> Option<String> {
    // First: form submit buttons
    for form in forms {
        if let Some(sel) = &form.submit_selector { return Some(sel.clone()); }
    }
    // Second: first visible, non-disabled button not in nav
    elements.iter()
        .find(|e| e.role == ElementRole::Button && e.is_visible && !e.is_disabled
            && e.parent_context.as_deref() != Some("nav"))
        .map(|e| e.selector.clone())
}

/// Detect search input.
fn detect_search_input(elements: &[SemanticElement]) -> Option<String> {
    elements.iter()
        .find(|e| e.role == ElementRole::SearchInput
            || e.label.to_lowercase().contains("search")
            || e.aria.get("aria-label").map_or(false, |l| l.to_lowercase().contains("search")))
        .map(|e| e.selector.clone())
}

/// Detect guards (cookie banners, modals) that block the main content.
fn detect_guards(elements: &[SemanticElement], html: &str) -> Vec<SemanticElement> {
    let lower = html.to_lowercase();
    let mut guards = Vec::new();

    // Cookie consent buttons
    for el in elements {
        let label_lower = el.label.to_lowercase();
        if (label_lower.contains("accept") || label_lower.contains("agree") || label_lower.contains("consent"))
            && (lower.contains("cookie") || lower.contains("consent") || lower.contains("privacy"))
        {
            guards.push(el.clone());
        }
    }

    // Dialog close buttons
    for el in elements {
        if el.role == ElementRole::Button
            && el.aria.get("aria-label").map_or(false, |l| l.to_lowercase().contains("close"))
            && el.parent_context.as_deref() == Some("dialog")
        {
            guards.push(el.clone());
        }
    }

    guards
}

fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        let after = &html[start + 7..];
        if let Some(end) = after.to_lowercase().find("</title>") {
            return after[..end].trim().to_string();
        }
    }
    String::new()
}

fn extract_form_name(form_html: &str) -> Option<String> {
    let lower = form_html.to_lowercase();
    for attr in &["name", "id", "aria-label"] {
        let pattern = format!("{attr}=\"");
        if let Some(pos) = lower.find(&pattern) {
            let start = pos + pattern.len();
            if let Some(end) = lower[start..].find('"') {
                let val = form_html[start..start + end].trim().to_string();
                if !val.is_empty() { return Some(val); }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom_parser;

    #[test]
    fn detect_login_form() {
        let json = r#"[
            {"tag":"input","type":"email","text":"","name":"email","id":null,"href":null,"placeholder":"Email"},
            {"tag":"input","type":"password","text":"","name":"password","id":null,"href":null,"placeholder":"Password"},
            {"tag":"button","type":"submit","text":"Sign in","name":null,"id":"login-btn","href":null,"placeholder":null}
        ]"#;
        let html = "<form id='login'><input name='email' type='email'><input name='password' type='password'><button id='login-btn'>Sign in</button></form>";
        let elements = dom_parser::parse_page(json, html);
        let constitution = build_constitution(elements, html, "https://example.com/login");
        assert!(!constitution.forms.is_empty());
        assert_eq!(constitution.forms[0].form_type, FormIntent::Login);
    }

    #[test]
    fn detect_search_form() {
        let json = r#"[{"tag":"input","type":"search","text":"","name":"q","id":"search","href":null,"placeholder":"Search"}]"#;
        let html = "<input id='search' type='search' name='q' placeholder='Search'>";
        let elements = dom_parser::parse_page(json, html);
        let constitution = build_constitution(elements, html, "https://example.com");
        assert!(constitution.search_input.is_some());
    }

    #[test]
    fn detect_cookie_guard() {
        let json = r#"[{"tag":"button","type":null,"text":"Accept cookies","name":null,"id":"cookie-btn","href":null,"placeholder":null}]"#;
        let html = "<div class='cookie-banner'><button id='cookie-btn'>Accept cookies</button></div>";
        let elements = dom_parser::parse_page(json, html);
        let constitution = build_constitution(elements, html, "https://example.com");
        assert!(!constitution.guards.is_empty());
    }
}
