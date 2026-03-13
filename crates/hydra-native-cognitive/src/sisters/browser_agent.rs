//! Browser Agent — multi-step web browsing pipeline.
//!
//! Makes Hydra the best internet browser agent by exposing Vision sister's
//! full capability stack: DOM extraction, form submission, action discovery,
//! grammar learning, intent-scoped perception, and multi-step navigation.
//!
//! Architecture: Navigate → Observe → Decide → Act → Observe → Loop

use super::connection::extract_text;
use super::cognitive::Sisters;
use hydra_native_state::utils::safe_truncate;

/// Result of a browse step — what did we see?
#[derive(Debug, Clone)]
pub struct BrowseObservation {
    pub url: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub interactive_elements: Option<String>,
    pub forms: Option<String>,
    pub links: Option<String>,
    pub grammar_available: bool,
}

/// Result of a browse interaction.
#[derive(Debug, Clone)]
pub struct BrowseActionResult {
    pub success: bool,
    pub new_url: Option<String>,
    pub response_content: Option<String>,
    pub error: Option<String>,
}

impl Sisters {
    // ═══════════════════════════════════════════════════════════════
    // LAYER 0: Navigate + Observe (DOM-first, zero vision tokens)
    // ═══════════════════════════════════════════════════════════════

    /// Navigate to a URL and extract DOM structure (L0 — zero vision tokens).
    /// Returns interactive elements, forms, links — everything needed to act.
    pub async fn browse_navigate(&self, url: &str) -> Option<BrowseObservation> {
        let vision = self.vision.as_ref()?;

        // L0: DOM extraction — interactive elements, forms, structure
        let dom_result = vision.call_tool("vision_dom_extract", serde_json::json!({
            "url": url,
            "fields": ["interactive", "forms", "links", "headings", "content"],
            "selectors": [],
        })).await.ok()?;

        let content = extract_text(&dom_result);
        let title = dom_result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
        let interactive = dom_result.get("interactive")
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default());
        let forms = dom_result.get("forms")
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default());
        let links = dom_result.get("links")
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default());
        let grammar = dom_result.get("grammar_available")
            .and_then(|v| v.as_bool()).unwrap_or(false);

        Some(BrowseObservation {
            url: url.to_string(),
            title,
            content: if content.is_empty() { None } else { Some(content) },
            interactive_elements: interactive,
            forms,
            links,
            grammar_available: grammar,
        })
    }

    /// Navigate and observe with intent-scoped perception (L2 — ~100 tokens).
    /// More focused than full DOM: extracts only what matches the intent.
    pub async fn browse_with_intent(&self, url: &str, intent: &str) -> Option<BrowseObservation> {
        let vision = self.vision.as_ref()?;

        let result = vision.call_tool("vision_intent_extract", serde_json::json!({
            "url": url,
            "intent": intent,
        })).await.ok()?;

        let content = extract_text(&result);
        let title = result.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());

        Some(BrowseObservation {
            url: url.to_string(),
            title,
            content: if content.is_empty() { None } else { Some(content) },
            interactive_elements: None,
            forms: None,
            links: None,
            grammar_available: result.get("grammar_hit").and_then(|v| v.as_bool()).unwrap_or(false),
        })
    }

    // ═══════════════════════════════════════════════════════════════
    // LAYER 1: Interact (forms, clicks, API calls)
    // ═══════════════════════════════════════════════════════════════

    /// Submit a form on the current page.
    /// fields: key-value pairs to fill in the form.
    pub async fn browse_submit_form(
        &self, url: &str, form_selector: &str, fields: &serde_json::Value,
    ) -> Option<BrowseActionResult> {
        let vision = self.vision.as_ref()?;

        let result = vision.call_tool("vision_dom_extract", serde_json::json!({
            "url": url,
            "action": "submit_form",
            "form_selector": form_selector,
            "fields": fields,
        })).await.ok()?;

        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let new_url = result.get("redirect_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let content = extract_text(&result);

        Some(BrowseActionResult {
            success,
            new_url,
            response_content: if content.is_empty() { None } else { Some(content) },
            error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    /// Click an interactive element by CSS selector.
    pub async fn browse_click(&self, url: &str, selector: &str) -> Option<BrowseActionResult> {
        let vision = self.vision.as_ref()?;

        let result = vision.call_tool("vision_dom_extract", serde_json::json!({
            "url": url,
            "action": "click",
            "selector": selector,
        })).await.ok()?;

        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let new_url = result.get("redirect_url").and_then(|v| v.as_str()).map(|s| s.to_string());
        let content = extract_text(&result);

        Some(BrowseActionResult {
            success, new_url,
            response_content: if content.is_empty() { None } else { Some(content) },
            error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    /// Execute a discovered API call (from action discovery).
    pub async fn browse_api_call(
        &self, url: &str, method: &str, endpoint: &str, body: Option<&serde_json::Value>,
    ) -> Option<BrowseActionResult> {
        let vision = self.vision.as_ref()?;

        let mut params = serde_json::json!({
            "url": url,
            "action": "api_call",
            "method": method,
            "endpoint": endpoint,
        });
        if let Some(b) = body {
            params["body"] = b.clone();
        }

        let result = vision.call_tool("vision_dom_extract", serde_json::json!(params)).await.ok()?;
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let content = extract_text(&result);

        Some(BrowseActionResult {
            success, new_url: None,
            response_content: if content.is_empty() { None } else { Some(content) },
            error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    // ═══════════════════════════════════════════════════════════════
    // LAYER 2: Visual perception (screenshots, OCR, captcha)
    // ═══════════════════════════════════════════════════════════════

    /// Take a screenshot of a page for visual analysis (L4 — ~400 tokens).
    pub async fn browse_screenshot(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_capture", serde_json::json!({
            "source": {"type": "url", "url": url},
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// OCR a page or element — extract text from visual content.
    pub async fn browse_ocr(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_ocr", serde_json::json!({
            "source": {"type": "url", "url": url},
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Handle captcha: screenshot → OCR → return text for solving.
    /// Returns the captcha image description + any extracted text.
    pub async fn browse_handle_captcha(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;

        // Use perception router to handle captcha intent
        let result = vision.call_tool("vision_perception_route", serde_json::json!({
            "url": url,
            "intent": "verify_captcha",
        })).await.ok()?;

        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // LAYER 3: Grammar learning (zero-token repeat visits)
    // ═══════════════════════════════════════════════════════════════

    /// Learn a site's grammar — CSS selectors, content patterns, intent routes.
    /// After learning, future visits to the same domain cost zero vision tokens.
    pub async fn browse_learn_grammar(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_grammar_learn", serde_json::json!({
            "url": url,
            "auto_discover": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Get grammar for a domain — check if we've learned its patterns.
    pub async fn browse_get_grammar(&self, domain: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_grammar_get", serde_json::json!({
            "domain": domain,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    // ═══════════════════════════════════════════════════════════════
    // LAYER 4: Auth flows (login, OAuth, API keys)
    // ═══════════════════════════════════════════════════════════════

    /// Login to a website with credentials.
    pub async fn browse_login(
        &self, url: &str, username: &str, password: &str,
    ) -> Option<BrowseActionResult> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_dom_extract", serde_json::json!({
            "url": url,
            "action": "login",
            "username": username,
            "password": password,
        })).await.ok()?;

        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let new_url = result.get("redirect_url").and_then(|v| v.as_str()).map(|s| s.to_string());

        Some(BrowseActionResult {
            success, new_url,
            response_content: Some(extract_text(&result)),
            error: result.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    // ═══════════════════════════════════════════════════════════════
    // LAYER 5: Web content mapping (full page extraction)
    // ═══════════════════════════════════════════════════════════════

    /// Map a web page — extract title, text content, links, metadata.
    /// Higher-level than DOM extraction: returns human-readable content.
    pub async fn browse_web_map(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_web_map", serde_json::json!({
            "url": url,
            "extract_text": true,
            "extract_links": true,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Diff two page states — what changed between visits?
    pub async fn browse_diff(&self, url: &str, baseline_id: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_diff", serde_json::json!({
            "baseline": baseline_id,
            "current": {"type": "url", "url": url},
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Search across captured web pages.
    pub async fn browse_search_captures(&self, query: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_query", serde_json::json!({
            "query": query,
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }

    /// Discover what actions are available on a page (forms, APIs, links).
    pub async fn browse_discover_actions(&self, url: &str) -> Option<String> {
        let vision = self.vision.as_ref()?;
        let result = vision.call_tool("vision_dom_extract", serde_json::json!({
            "url": url,
            "fields": ["forms", "api_endpoints", "interactive", "drag_drop"],
        })).await.ok()?;
        let text = extract_text(&result);
        if text.is_empty() { None } else { Some(text) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browse_observation_debug() {
        let obs = BrowseObservation {
            url: "https://example.com".into(),
            title: Some("Example".into()),
            content: Some("Hello world".into()),
            interactive_elements: None,
            forms: None,
            links: None,
            grammar_available: false,
        };
        assert_eq!(obs.url, "https://example.com");
        assert!(obs.title.is_some());
    }

    #[test]
    fn test_browse_action_result_debug() {
        let r = BrowseActionResult {
            success: true,
            new_url: Some("https://example.com/dashboard".into()),
            response_content: None,
            error: None,
        };
        assert!(r.success);
        assert!(r.new_url.is_some());
    }
}
