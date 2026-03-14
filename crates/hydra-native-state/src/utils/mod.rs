//! Utility functions — formatting, plan extraction, deliverable steps.

pub mod format;
pub mod markdown;
pub mod plan;

pub use format::{detect_language, format_bytes, home_dir, hydra_data_dir, safe_truncate, strip_emojis};
pub use plan::{extract_json_plan, generate_deliverable_steps};
