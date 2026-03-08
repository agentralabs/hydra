//! Sister MCP integration — connection management and cognitive dispatch.

pub mod connection;
pub mod cognitive;

pub use cognitive::{init_sisters, Sisters, SistersHandle};
pub use connection::extract_text;
