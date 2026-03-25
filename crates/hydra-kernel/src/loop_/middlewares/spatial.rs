//! Spatial Presence + Document Vision middleware (O19 + O20).
//! O19: Webcam presence + gesture → enrichment. Camera off by default.
//! O20: Document detection → OCR/structural analysis → enrichment.

use hydra_desktop::{PresenceEngine, PresenceState, GestureCommand};
use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

/// Spatial presence middleware — integrates webcam presence into the loop.
pub struct SpatialMiddleware {
    engine: PresenceEngine,
    gesture_confirmations: u64,
}

impl SpatialMiddleware {
    pub fn new() -> Self {
        Self { engine: PresenceEngine::new(), gesture_confirmations: 0 }
    }
}

impl Default for SpatialMiddleware {
    fn default() -> Self { Self::new() }
}

impl CycleMiddleware for SpatialMiddleware {
    fn name(&self) -> &'static str { "spatial" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Detect enable/disable commands in user input
        let lower = perceived.raw.to_lowercase();
        if lower.contains("enable camera") || lower.contains("camera on") {
            if let Err(e) = self.engine.enable() {
                eprintln!("hydra-spatial: enable failed: {e}");
            }
        }
        if lower.contains("disable camera") || lower.contains("camera off") {
            self.engine.disable();
        }

        // Poll presence engine
        let (changed, cmd) = self.engine.poll();
        if changed {
            perceived.enrichments.insert(
                "spatial.presence".into(), self.engine.state().label().into());
        }
        // Handle gesture commands
        if let Some(GestureCommand::Confirm) = cmd {
            self.gesture_confirmations += 1;
            perceived.enrichments.insert("spatial.gesture".into(), "confirm".into());
        }
        if let Some(GestureCommand::Attention) = cmd {
            perceived.enrichments.insert("spatial.gesture".into(), "attention — user returned".into());
        }

        // O20: Document vision — detect file references, process documents
        if let Some(path) = extract_file_reference(&perceived.raw) {
            if looks_like_document(&path) {
                match hydra_desktop::document::process_document(&path) {
                    Ok(content) => {
                        perceived.enrichments.insert("document.content".into(),
                            hydra_desktop::document::summarize_content(&content));
                        perceived.enrichments.insert("document.type".into(),
                            content.doc_type.label().into());
                        eprintln!("hydra-spatial: document processed (tier {})", content.tier_used);
                    }
                    Err(e) => eprintln!("hydra-spatial: document: {e}"),
                }
            }
        }
    }

    fn enrich_prompt(&self, perceived: &PerceivedInput) -> Vec<String> {
        let mut lines = Vec::new();
        match self.engine.state() {
            PresenceState::Away => lines.push("[Spatial] User appears away. Keep brief.".into()),
            PresenceState::Idle => lines.push("[Spatial] User idle. Suggest next steps.".into()),
            _ => {}
        }
        // O20: Document context for LLM
        if let Some(doc) = perceived.enrichments.get("document.content") {
            lines.push(format!("[Document] Analyze this content:\n{}", &doc[..doc.len().min(400)]));
        }
        lines
    }

    fn post_deliver(&mut self, _cycle: &CycleResult) {
        // Feed gesture confirmations to genome via feedback loop (Law 10 Check 3)
        if self.gesture_confirmations > 0 && self.gesture_confirmations % 5 == 0 {
            crate::feedback::log_outcome(&crate::feedback::ActionOutcome::Success {
                approach: "gesture_confirmation".into(),
                domain: "spatial-presence".into(),
                duration_ms: 0,
                quality: 1.0,
            });
        }
    }
}

// ── O20 Helpers ──

/// Extract a file path reference from user input.
fn extract_file_reference(input: &str) -> Option<String> {
    for word in input.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
        if clean.contains('.') && (clean.contains('/') || clean.contains('\\')
            || clean.ends_with(".pdf") || clean.ends_with(".png") || clean.ends_with(".jpg")
            || clean.ends_with(".csv") || clean.ends_with(".jpeg"))
        {
            if std::path::Path::new(clean).exists() { return Some(clean.to_string()); }
        }
    }
    None
}

/// Check if a path looks like a document (image, PDF, CSV).
fn looks_like_document(path: &str) -> bool {
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "pdf" | "png" | "jpg" | "jpeg" | "csv" | "tsv" | "bmp" | "tiff")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middleware_name() {
        let mw = SpatialMiddleware::new();
        assert_eq!(mw.name(), "spatial");
    }

    #[test]
    fn starts_disabled() {
        let mw = SpatialMiddleware::new();
        assert_eq!(*mw.engine.state(), PresenceState::Disabled);
    }
}
