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

/// Parse LLM response text, extract any <computer_use> or inline action tags.
pub fn parse_response(raw: &str) -> (String, Vec<ParsedAction>) {
    let mut clean = String::new();
    let mut actions = Vec::new();
    let mut remaining = raw;

    // Parse <computer_use> blocks
    while let Some(start) = remaining.find("<computer_use>") {
        clean.push_str(&remaining[..start]);
        if let Some(end) = remaining.find("</computer_use>") {
            let tag_content = &remaining[start + 14..end];
            if let Some(action) = parse_tag(tag_content) { actions.push(action); }
            remaining = &remaining[end + 15..];
        } else { remaining = &remaining[start + 14..]; }
    }
    clean.push_str(remaining);

    // Also parse inline action tags: <click>x,y</click>, <screenshot>, <type>text</type>
    let inline_tags = [("click", "click"), ("screenshot", "screenshot"),
        ("type", "type"), ("key", "key_press"), ("scroll", "scroll")];
    for (tag, action_type) in &inline_tags {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let mut search = clean.as_str();
        while let Some(s) = search.find(&open) {
            let after = &search[s + open.len()..];
            let target = if let Some(e) = after.find(&close) {
                let t = after[..e].trim().to_string();
                search = &after[e + close.len()..];
                t
            } else { search = &search[s + open.len()..]; continue; };
            if !target.is_empty() || *tag == "screenshot" {
                actions.push(ParsedAction {
                    action_type: action_type.to_string(),
                    target: target.clone(),
                    display: format!("⏵ {action_type}: {target}"),
                });
            }
        }
    }
    // Strip inline tags from clean text
    let mut result = clean.clone();
    for (tag, _) in &inline_tags {
        let re_open = format!("<{tag}>");
        let re_close = format!("</{tag}>");
        result = result.replace(&re_open, "").replace(&re_close, "");
    }
    let result = result.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n");
    (result, actions)
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

/// Execute a parsed action — actually do it on the system.
pub fn execute_action(action: &ParsedAction) -> String {
    match action.action_type.as_str() {
        "browser_navigate" => {
            // Open URL in the user's default browser (visible, not headless)
            let url = &action.target;
            let result = if cfg!(target_os = "macos") {
                std::process::Command::new("open").arg(url).status()
            } else if cfg!(target_os = "linux") {
                std::process::Command::new("xdg-open").arg(url).status()
            } else {
                return "Unsupported platform".into();
            };
            match result {
                Ok(s) if s.success() => format!("✓ Opened {}", shorten_url(url)),
                Ok(s) => format!("✗ Failed to open (exit {})", s.code().unwrap_or(-1)),
                Err(e) => format!("✗ Error: {e}"),
            }
        }
        "click" => {
            let mut input = hydra_desktop::InputSimulator::new();
            // Parse coordinates from "[x, y]" format
            let coords = action.target.trim_matches(|c| c == '[' || c == ']');
            let parts: Vec<&str> = coords.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    match input.click_at(x, y) {
                        Ok(_) => format!("✓ Clicked at ({x:.0}, {y:.0})"),
                        Err(e) => format!("✗ Click failed: {e}"),
                    }
                } else { "✗ Invalid coordinates".into() }
            } else { "✗ Invalid coordinate format".into() }
        }
        "type" => {
            let input = hydra_desktop::InputSimulator::new();
            match input.key_type(&action.target) {
                Ok(_) => format!("✓ Typed: \"{}\"", &action.target[..action.target.len().min(20)]),
                Err(e) => format!("✗ Type failed: {e}"),
            }
        }
        "key_press" | "key_combo" => {
            let input = hydra_desktop::InputSimulator::new();
            if action.target.contains('+') {
                let parts: Vec<&str> = action.target.split('+').collect();
                if parts.len() == 2 {
                    match input.key_combo(parts[0].trim(), parts[1].trim()) {
                        Ok(_) => format!("✓ Key: {}", action.target),
                        Err(e) => format!("✗ Key failed: {e}"),
                    }
                } else { "✗ Invalid key combo".into() }
            } else {
                match input.key_press(&action.target) {
                    Ok(_) => format!("✓ Key: {}", action.target),
                    Err(e) => format!("✗ Key failed: {e}"),
                }
            }
        }
        "screenshot" => {
            match hydra_desktop::ScreenCapture::capture_full() {
                Ok((bytes, info)) => {
                    let path = std::env::temp_dir().join("hydra-screenshot.png");
                    match std::fs::write(&path, &bytes) {
                        Ok(_) => format!("✓ Screenshot ({}x{}, {}KB) → {}", info.width, info.height, bytes.len()/1024, path.display()),
                        Err(e) => format!("✗ Save failed: {e}"),
                    }
                }
                Err(e) => format!("✗ Screenshot failed: {e}"),
            }
        }
        "scroll" => {
            let mut input = hydra_desktop::InputSimulator::new();
            let amount = action.target.parse::<i32>().unwrap_or(3);
            match input.scroll_wheel(960.0, 540.0, amount) {
                Ok(_) => format!("✓ Scrolled {amount}"),
                Err(e) => format!("✗ Scroll failed: {e}"),
            }
        }
        "key" => {
            // Alias for key_press
            let input = hydra_desktop::InputSimulator::new();
            match input.key_press(&action.target) {
                Ok(_) => format!("✓ Key: {}", action.target),
                Err(e) => format!("✗ Key failed: {e}"),
            }
        }
        _ => format!("⏵ {}: not yet wired", action.action_type),
    }
}
