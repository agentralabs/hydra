//! LLM capability probing and request routing.
//!
//! Phase 4, Part D: Probes the connected LLM to discover its real capabilities
//! and builds optimal requests for each model's strengths.

pub mod capability_probe;

pub use capability_probe::{LLMCapabilityProfile, ToolFormat};
