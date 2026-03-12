//! Pre-phase intent dispatch handlers — extracted from loop_runner.rs for compilation performance.
//!
//! Each handler returns `true` if it handled the intent (caller should return early),
//! or `false` to fall through to the next handler / 5-phase loop.
//!
//! Split into sub-modules for compilation performance:
//! - `dispatch_intents`: greeting, farewell, memory recall, settings, memory store
//! - `dispatch_actions`: crystallized skills, dep queries, slash commands, direct actions

pub(crate) use super::dispatch_intents::{
    handle_greeting_farewell_thanks,
    handle_memory_recall,
    handle_settings,
    handle_memory_store,
};

pub(crate) use super::dispatch_actions::{
    handle_crystallized_skill,
    handle_dep_query_precheck,
    handle_slash_command,
    handle_direct_action,
    handle_project_exec_natural,
};
