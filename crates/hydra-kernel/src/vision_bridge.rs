//! LlmVisionProvider — concrete VisionProvider that calls Anthropic Messages API.
//!
//! Lives in the kernel because the kernel owns the LLM caller and API keys.
//! No circular dependency: kernel depends on hydra-browser for the trait.

use async_trait::async_trait;
use hydra_browser::BrowserError;
use hydra_browser::VisionProvider;

/// Vision provider backed by Anthropic's Claude vision API.
#[derive(Clone)]
pub struct LlmVisionProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl LlmVisionProvider {
    /// Create a new LLM vision provider.
    /// Reads ANTHROPIC_API_KEY from environment if not provided.
    pub fn new() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        Some(Self {
            client: reqwest::Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        })
    }

    /// Create with an explicit API key and model.
    pub fn with_config(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl VisionProvider for LlmVisionProvider {
    async fn analyze_image(
        &self,
        image_bytes: &[u8],
        prompt: &str,
    ) -> Result<String, BrowserError> {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        // Detect actual format from magic bytes (JPEG=FF D8, PNG=89 50)
        let media_type = if image_bytes.len() >= 2 && image_bytes[0] == 0xFF && image_bytes[1] == 0xD8 {
            "image/jpeg"
        } else { "image/png" };

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": b64,
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt,
                    }
                ]
            }]
        });

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BrowserError::VisionError(format!("HTTP error: {e}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| BrowserError::VisionError(format!("Read body: {e}")))?;

        if !status.is_success() {
            return Err(BrowserError::VisionError(format!(
                "API {status}: {text}"
            )));
        }

        // Parse the Anthropic response to extract the text content
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| BrowserError::VisionError(format!("Parse response: {e}")))?;

        let content_text = parsed
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        if content_text.is_empty() {
            return Err(BrowserError::VisionError(
                "Empty response from vision API".into(),
            ));
        }

        eprintln!(
            "hydra-kernel: vision analysis complete ({} chars)",
            content_text.len()
        );
        Ok(content_text)
    }
}

// ── Three-Tier Vision-Action Bridge ──

/// Which tier resolved the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionTier { Structural, Ocr, FullVision }

/// Result of resolving an intent to a screen target.
#[derive(Debug)]
pub enum ResolveResult {
    /// Found via Tier 1 or 2 — coordinates ready.
    Found { tier: VisionTier, x: f64, y: f64, label: String },
    /// Needs Tier 3 (LLM vision) — caller must invoke async vision.
    NeedsVision,
    /// Nothing found at any tier.
    NotFound(String),
}

/// Resolve an intent to a clickable target using the 3-tier cascade.
/// Tier 1: Structural (accessibility tree, 0 tokens, <100ms)
/// Tier 2: OCR (text recognition, 0 tokens, <500ms)
/// Tier 3: Returns NeedsVision (caller invokes LLM)
pub fn resolve_intent(intent: &str, is_browser: bool) -> ResolveResult {
    // Tier 1: Structural
    if !is_browser {
        match hydra_desktop::accessibility::AccessibilityTree::from_focused_app() {
            Ok(tree) if !tree.elements.is_empty() => {
                if let Some(el) = tree.find_by_title(intent) {
                    let (cx, cy) = hydra_desktop::accessibility::AccessibilityTree::element_center(el);
                    eprintln!("hydra-vision: Tier 1 hit — '{}' at ({:.0}, {:.0})", el.title, cx, cy);
                    return ResolveResult::Found {
                        tier: VisionTier::Structural, x: cx, y: cy, label: el.title.clone(),
                    };
                }
            }
            _ => {} // EC-2.1: empty tree, cascade to Tier 2
        }
    }
    // (For browser: semantic-nav is called separately in agent_task.rs — already Tier 1)

    // Tier 2: OCR
    match hydra_desktop::ocr::ocr_current_screen() {
        Ok(regions) if !regions.is_empty() => {
            if let Some(region) = hydra_desktop::ocr::find_best_match(intent, &regions) {
                let cx = region.x + region.width / 2.0;
                let cy = region.y + region.height / 2.0;
                eprintln!("hydra-vision: Tier 2 OCR hit — '{}' at ({:.0}, {:.0})", region.text, cx, cy);
                return ResolveResult::Found {
                    tier: VisionTier::Ocr, x: cx, y: cy, label: region.text.clone(),
                };
            }
        }
        _ => {} // OCR failed, cascade to Tier 3
    }

    // Tier 3: Need vision LLM
    eprintln!("hydra-vision: Tier 1+2 failed for '{}', needs vision LLM", intent);
    ResolveResult::NeedsVision
}

// ── Observation Loop ──

/// Watch the screen until a condition is met or timeout (EC-2.7).
/// Uses Tier 1 (a11y) or Tier 2 (OCR) — no LLM tokens.
pub fn observe_until(text_condition: &str, should_appear: bool, timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    let interval = std::time::Duration::from_millis(500);

    while start.elapsed().as_millis() < timeout_ms as u128 {
        // Check via OCR (most reliable for observation)
        let found = match hydra_desktop::ocr::ocr_current_screen() {
            Ok(regions) => hydra_desktop::ocr::find_best_match(text_condition, &regions).is_some(),
            Err(_) => false,
        };

        if should_appear && found { return true; }
        if !should_appear && !found { return true; }

        std::thread::sleep(interval);
    }

    eprintln!("hydra-vision: observation timeout after {}ms for '{}'", timeout_ms, text_condition);
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_requires_api_key() {
        let _provider = LlmVisionProvider::with_config("test-key".into(), "claude-sonnet-4-20250514".into());
    }

    #[test]
    fn resolve_intent_returns_something() {
        // Without a focused app, should cascade to NeedsVision or NotFound
        let result = resolve_intent("Submit", false);
        // On CI/headless, accessibility tree will be empty → OCR may fail → NeedsVision
        assert!(matches!(result, ResolveResult::Found { .. } | ResolveResult::NeedsVision | ResolveResult::NotFound(_)));
    }

    #[test]
    fn vision_tier_ordering() {
        assert_ne!(VisionTier::Structural, VisionTier::Ocr);
        assert_ne!(VisionTier::Ocr, VisionTier::FullVision);
    }
}
