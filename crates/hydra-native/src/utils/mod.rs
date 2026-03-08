//! Utility functions — formatting, plan extraction, deliverable steps.

pub mod format;
pub mod markdown;
pub mod plan;

pub use format::{detect_language, format_bytes};
pub use plan::{extract_json_plan, generate_deliverable_steps};
