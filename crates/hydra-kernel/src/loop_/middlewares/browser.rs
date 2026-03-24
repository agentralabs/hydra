//! Browser/IO middleware — browser, voice, protocol, device awareness.
//!
//! Uses LLM intent classifier instead of hardcoded keywords.
//! Falls back to heuristic when LLM is unavailable.

use crate::intent_classifier;
use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

pub struct BrowserMiddleware {
    browser_actions_observed: u64,
    api_key: Option<String>,
}

impl BrowserMiddleware {
    pub fn new() -> Self {
        Self {
            browser_actions_observed: 0,
            api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }
}

impl Default for BrowserMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl CycleMiddleware for BrowserMiddleware {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        let input = &perceived.raw;

        // Use heuristic classifier (synchronous path — LLM path used at TUI level).
        // The TUI can override with the async LLM-based classifier.
        let intent = intent_classifier::classify_heuristic_sync(input, self.api_key.as_deref());
        intent_classifier::inject_enrichments(&intent, &mut perceived.enrichments);
    }

    fn enrich_prompt(&self, perceived: &PerceivedInput) -> Vec<String> {
        let mut context = Vec::new();

        if perceived.enrichments.contains_key("browser_relevant") {
            context.push(
                "[Browser] You have ACTIVE browser capability. When the user asks you to open, visit, \
                 or browse a URL, you ARE doing it — the browser automation runs in parallel. \
                 Do NOT say you cannot browse. Discuss the content you find.".into()
            );
        }

        let intent = perceived.enrichments.get("agent_intent").map(|s| s.as_str());
        match intent {
            Some("shell") => context.push(
                "[Shell] You have shell access. When the user asks you to run a command, \
                 the executor runs it in parallel.".into()
            ),
            Some("desktop") => context.push(
                "[Desktop] You have desktop automation capability. You can control \
                 applications via screen capture and input simulation.".into()
            ),
            Some("file") => context.push(
                "[Files] You have file system access. You can read, write, and search files.".into()
            ),
            _ => {}
        }

        context
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        if cycle.response.contains("browser:")
            || cycle.response.contains("navigated")
            || cycle.response.contains("screenshot")
        {
            self.browser_actions_observed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middleware_name() {
        let mw = BrowserMiddleware::new();
        assert_eq!(mw.name(), "browser");
    }
}
