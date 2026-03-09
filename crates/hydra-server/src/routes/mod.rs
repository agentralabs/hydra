pub mod messages;
pub mod profile;
pub mod push;
pub mod runs;
pub mod system;
pub mod tasks;

use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::Router;

pub use messages::MessageRoutes;
pub use profile::ProfileRoutes;
pub use push::PushRoutes;
pub use runs::RunRoutes;
pub use system::SystemRoutes;
pub use tasks::TaskRoutes;

use crate::state::AppState;

/// Build a sub-router containing all REST API routes
pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // ── Profile ──────────────────────────────────────────
        .route(
            ProfileRoutes::get_profile(),
            get(profile::get_profile),
        )
        .route(
            ProfileRoutes::update_profile(),
            put(profile::update_profile),
        )
        .route(
            ProfileRoutes::set_name(),
            put(profile::set_name),
        )
        .route(
            ProfileRoutes::enable_voice(),
            post(profile::enable_voice),
        )
        .route(
            ProfileRoutes::complete_onboarding(),
            post(profile::complete_onboarding),
        )
        .route(
            ProfileRoutes::get_greeting(),
            get(profile::get_greeting),
        )
        .route(
            ProfileRoutes::is_first_run(),
            get(profile::is_first_run),
        )
        // ── Conversations / Messages ─────────────────────────
        .route(
            MessageRoutes::list_conversations(),
            get(messages::list_conversations).post(messages::create_conversation),
        )
        .route(
            MessageRoutes::get_conversation(),
            get(messages::get_conversation).delete(messages::delete_conversation),
        )
        .route(
            MessageRoutes::list_messages(),
            get(messages::list_messages).post(messages::send_message),
        )
        .route(
            MessageRoutes::get_message(),
            get(messages::get_message),
        )
        .route(
            MessageRoutes::retry_message(),
            post(messages::retry_message),
        )
        .route(
            "/api/search",
            get(messages::search_messages),
        )
        // ── Tasks ────────────────────────────────────────────
        .route(
            TaskRoutes::list_tasks(),
            get(tasks::list_tasks).post(tasks::create_task),
        )
        .route(
            TaskRoutes::get_task(),
            get(tasks::get_task)
                .put(tasks::update_task)
                .delete(tasks::delete_task),
        )
        .route(
            TaskRoutes::pause_task(),
            post(tasks::pause_task),
        )
        .route(
            TaskRoutes::resume_task(),
            post(tasks::resume_task),
        )
        .route(
            TaskRoutes::task_status(),
            get(tasks::task_status),
        )
        .route(
            TaskRoutes::list_subtasks(),
            get(tasks::list_subtasks),
        )
        // ── Runs ──────────────────────────────────────────────
        .route(
            RunRoutes::list_runs(),
            get(runs::list_runs).post(runs::execute_run_handler),
        )
        .route(
            RunRoutes::get_run(),
            get(runs::get_run),
        )
        // ── Run actions (RPC parity) ─────────────────────────
        .route(
            SystemRoutes::cancel_run(),
            post(system::cancel_run),
        )
        .route(
            SystemRoutes::approve_run(),
            post(system::approve_run),
        )
        .route(
            SystemRoutes::run_status(),
            get(system::run_status),
        )
        .route(
            SystemRoutes::kill_run(),
            post(system::kill_run),
        )
        // ── Models ────────────────────────────────────────────
        .route(
            RunRoutes::list_models(),
            get(runs::list_models),
        )
        // ── Push Notifications ────────────────────────────────
        .route(
            PushRoutes::register_device(),
            post(push::register_device),
        )
        .route(
            PushRoutes::unregister_device(),
            delete(push::unregister_device),
        )
        .route(
            PushRoutes::list_devices(),
            get(push::list_devices),
        )
        .route(
            PushRoutes::test_push(),
            post(push::test_push),
        )
        .route(
            PushRoutes::subscribe_sse(),
            get(push::subscribe_sse),
        )
        // ── System ────────────────────────────────────────────
        .route(
            SystemRoutes::system_status(),
            get(system::system_status),
        )
        .route(
            SystemRoutes::list_steps(),
            get(system::list_steps),
        )
        // ── Approvals ─────────────────────────────────────────
        .route(
            SystemRoutes::list_approvals(),
            get(system::list_approvals),
        )
        .route(
            SystemRoutes::approve(),
            post(system::approve_approval),
        )
        .route(
            SystemRoutes::deny(),
            post(system::deny_approval),
        )
        // ── Sprint 1-3 engines ───────────────────────────────
        .route(
            SystemRoutes::trust(),
            get(system::get_trust),
        )
        .route(
            SystemRoutes::inventions(),
            get(system::get_inventions),
        )
        .route(
            SystemRoutes::budget(),
            get(system::get_budget),
        )
        .route(
            SystemRoutes::receipts(),
            get(system::get_receipts),
        )
        .route(
            SystemRoutes::offline(),
            get(system::get_offline),
        )
}
