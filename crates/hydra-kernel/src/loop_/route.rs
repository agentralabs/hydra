//! route.rs — Path selection from PerceivedInput.
//! Pure logic — no I/O, no failures.

use hydra_genome::GenomeStore;
use hydra_reasoning::ReasoningEngine;

use crate::loop_::types::{PerceivedInput, RoutePath};

pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self
    }

    /// Select path and attempt zero-token resolution.
    /// Returns (path, Option<resolved_text>).
    pub fn route(
        &self,
        perceived: &PerceivedInput,
        genome: &GenomeStore,
    ) -> (RoutePath, Option<String>) {
        let raw = &perceived.raw;
        let conf = perceived.comprehended.confidence;
        let lower = raw.to_lowercase();

        // Code/complex keywords -> always LLM long path
        let is_code_task = lower.contains("implement")
            || lower.contains("write")
            || lower.contains("build")
            || lower.contains("fix")
            || lower.contains("debug")
            || lower.contains("refactor")
            || lower.contains("migrate")
            || lower.contains("function")
            || lower.contains("struct")
            || raw.len() > 400;

        if is_code_task {
            return (RoutePath::LlmLong, None);
        }

        // High confidence -> try reasoning engine for zero-token resolution
        if conf >= 0.60 && perceived.attention.has_focus() {
            let engine = ReasoningEngine::new();
            match engine.reason(&perceived.comprehended, &perceived.attention, genome) {
                Ok(result) if !result.conclusions.is_empty() => {
                    let text = result.primary.as_ref().map(|c| c.statement.clone());

                    if conf >= 0.80 {
                        if let Some(t) = text {
                            return (
                                RoutePath::ZeroToken {
                                    reason: format!("conf={conf:.2}"),
                                },
                                Some(t),
                            );
                        }
                    }
                    // Medium confidence = reasoning path
                    return (
                        RoutePath::Reasoning {
                            mode: result
                                .primary
                                .as_ref()
                                .map(|c| c.mode.label().to_string())
                                .unwrap_or_else(|| "deductive".into()),
                        },
                        result.primary.map(|c| c.statement),
                    );
                }
                Err(e) => {
                    tracing::debug!("reasoning error: {:?}", e);
                }
                Ok(_) => {
                    // No conclusions — fall through to LLM
                }
            }
        }

        // Default: LLM short
        (RoutePath::LlmShort, None)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
