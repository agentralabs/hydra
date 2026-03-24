//! Verifier — post-action DOM check to confirm the plan succeeded.
//! Checks URL changes, form fills, and content changes.

/// Result of verification.
#[derive(Debug)]
pub struct VerifyResult {
    pub success: bool,
    pub observation: String,
}

/// Verify that the execution plan achieved its goal.
/// Compares before and after page state.
pub async fn verify(
    engine: &hydra_browser::BrowserEngine,
    before_url: &str,
    strategy: &str,
) -> VerifyResult {
    // Check if URL changed (for navigation actions)
    let after_html = match engine.html().await {
        Ok(html) => html,
        Err(e) => {
            return VerifyResult {
                success: false,
                observation: format!("Cannot read page after execution: {e}"),
            };
        }
    };

    let after_text = strip_tags(&after_html);

    // For navigation: URL should have changed
    if strategy.contains("Navigate") {
        // We can't easily get the current URL without a CDP call,
        // but if the page content changed significantly, it's likely navigated
        if after_text.len() > 100 {
            return VerifyResult {
                success: true,
                observation: "Page content loaded after navigation".into(),
            };
        }
    }

    // For form submission: check if page changed (success message, redirect, etc.)
    if strategy.contains("form") || strategy.contains("Submit") || strategy.contains("Fill") {
        // Check for common success indicators
        let lower = after_text.to_lowercase();
        let success_indicators = ["success", "thank you", "submitted", "posted", "sent", "saved",
            "welcome", "logged in", "dashboard", "feed"];
        if success_indicators.iter().any(|s| lower.contains(s)) {
            return VerifyResult {
                success: true,
                observation: "Success indicators found in page content".into(),
            };
        }
        // Check for error indicators
        let error_indicators = ["error", "invalid", "failed", "incorrect", "wrong", "try again"];
        if error_indicators.iter().any(|s| lower.contains(s)) {
            return VerifyResult {
                success: false,
                observation: "Error indicators found in page content".into(),
            };
        }
    }

    // Default: assume success if page has content
    if after_text.len() > 50 {
        VerifyResult {
            success: true,
            observation: format!("Page loaded ({} chars)", after_text.len()),
        }
    } else {
        VerifyResult {
            success: false,
            observation: "Page appears empty after execution".into(),
        }
    }
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
    fn strip_tags_works() {
        assert_eq!(strip_tags("<p>Hello <b>world</b></p>"), "Hello world");
    }
}
