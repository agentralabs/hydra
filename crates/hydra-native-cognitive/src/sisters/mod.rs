//! Sister MCP integration — connection management and cognitive dispatch.

pub mod connection;
pub mod cognitive;
pub mod cognitive_prompt;
pub mod cognitive_prompt_sections;
pub mod delegation;
pub mod learn;
pub mod perceive;

// Phase 5.5 — Deep Sister Integration
pub mod memory_deep;
pub mod contract_deep;
pub mod planning_deep;
pub mod reality_deep;
pub mod aegis_deep;
pub mod comm_deep;
pub mod extras_deep;

pub use cognitive::{init_sisters, Sisters, SistersHandle};
pub use connection::extract_text;
