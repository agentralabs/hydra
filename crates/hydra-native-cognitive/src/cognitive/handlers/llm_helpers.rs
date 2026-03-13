//! LLM utility functions — error diagnosis, retry logic, token management, tool routing.
//!
//! Split into submodules for maintainability:
//! - `llm_helpers_core`: error patterns, diagnostics, dependency detection
//! - `llm_helpers_review`: adaptive tokens, self-review, clarification questions
//! - `llm_helpers_commands`: slash commands, project detection, tool routing

#[path = "llm_helpers_core.rs"]
mod core;

#[path = "llm_helpers_review.rs"]
mod review;

#[path = "llm_helpers_commands.rs"]
mod commands;

#[path = "llm_tool_routing.rs"]
pub(crate) mod llm_tool_routing;

// Re-export everything at the original path
pub(crate) use core::recognize_error_pattern;
pub(crate) use core::diagnose_and_retry;
pub(crate) use core::commands_are_dependent;

pub(crate) use review::adaptive_max_tokens;
pub(crate) use review::self_review_response;
pub(crate) use review::generate_clarification_question;

pub(crate) use commands::extract_primary_topic;
pub(crate) use commands::detect_project_command;
pub(crate) use commands::handle_universal_slash_command;
pub(crate) use commands::route_tools_for_prompt;
pub(crate) use commands::format_tool_list;
