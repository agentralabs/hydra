//! Action Parser — intercepts <computer_use> XML from LLM responses.
//! Strips raw XML, routes to agents, returns compact notifications.
//! The user sees "⏵ Browser → google.com" not raw XML tags.

/// A parsed action from LLM response.
#[derive(Debug, Clone)]
pub struct ParsedAction {
    pub action_type: String,    // "browser_navigate", "click", "type", etc.
    pub target: String,         // URL, coordinates, text
    pub display: String,        // compact display string for TUI
}

/// Parse LLM response text, extract any <computer_use> actions, return clean text + actions.
pub fn parse_response(raw: &str) -> (String, Vec<ParsedAction>) {
    let mut clean = String::new();
    let mut actions = Vec::new();
    let mut remaining = raw;

    while let Some(start) = remaining.find("<computer_use>") {
        // Add text before the tag
        clean.push_str(&remaining[..start]);

        if let Some(end) = remaining.find("</computer_use>") {
            let tag_content = &remaining[start + 14..end];
            if let Some(action) = parse_tag(tag_content) {
                actions.push(action);
            }
            remaining = &remaining[end + 15..];
        } else {
            // Unclosed tag — skip it
            remaining = &remaining[start + 14..];
        }
    }
    clean.push_str(remaining);

    // Clean up extra whitespace from tag removal
    let clean = clean.lines()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    (clean, actions)
}

fn parse_tag(content: &str) -> Option<ParsedAction> {
    let action_type = extract_xml_value(content, "action")?;
    let url = extract_xml_value(content, "url");
    let text = extract_xml_value(content, "text");
    let coordinate = extract_xml_value(content, "coordinate");
    let key = extract_xml_value(content, "key");

    let (target, display) = match action_type.as_str() {
        "browser_navigate" => {
            let url = url.unwrap_or_default();
            (url.clone(), format!("⏵ Browser → {}", shorten_url(&url)))
        }
        "click" => {
            let coord = coordinate.unwrap_or_default();
            (coord.clone(), format!("⏵ Click at {coord}"))
        }
        "type" => {
            let t = text.unwrap_or_default();
            let short = if t.len() > 30 { format!("{}...", &t[..27]) } else { t.clone() };
            (t, format!("⏵ Type: \"{short}\""))
        }
        "key_press" | "key_combo" => {
            let k = key.or(text).unwrap_or_default();
            (k.clone(), format!("⏵ Key: {k}"))
        }
        "scroll" => ("scroll".into(), "⏵ Scroll".into()),
        "drag" => {
            let coord = coordinate.unwrap_or_default();
            (coord.clone(), format!("⏵ Drag from {coord}"))
        }
        other => (String::new(), format!("⏵ {other}")),
    };

    Some(ParsedAction { action_type, target, display })
}

fn extract_xml_value(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = content.find(&open)? + open.len();
    let end = content.find(&close)?;
    Some(content[start..end].trim().to_string())
}

fn shorten_url(url: &str) -> String {
    let clean = url.trim_start_matches("https://").trim_start_matches("http://");
    if clean.len() > 40 { format!("{}...", &clean[..37]) } else { clean.into() }
}
