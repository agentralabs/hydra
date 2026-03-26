//! v2 — Complete TUI rewrite (Claude Code style).
//! Event-Action-State architecture. No god objects. No scattered mutation.

pub mod action;
pub mod agent_task;
pub mod cache;
pub mod alert;
pub mod bridge_companion;
pub mod browser_task;
pub mod bridge_streaming;
pub mod bridge_voice;
pub mod commands;
pub mod config_schema;
pub mod dispatch;
pub mod enrichment_bridge;
pub mod modal;
pub mod morning_brief;
pub mod search_parse;
pub mod search_task;
pub mod session;
pub mod shell_task;
pub mod smart_pacer;
pub mod state;
pub mod tui_helpers;
pub mod view;
pub mod action_parser;
pub mod submit;
