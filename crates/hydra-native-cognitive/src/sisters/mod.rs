//! Sister MCP integration — connection management and cognitive dispatch.

pub mod connection;
pub mod cognitive;
pub mod cognitive_prompt;
pub mod cognitive_prompt_sections;
pub mod delegation;
pub mod learn;
pub mod perceive;

pub use cognitive::{init_sisters, Sisters, SistersHandle};
pub use connection::extract_text;
