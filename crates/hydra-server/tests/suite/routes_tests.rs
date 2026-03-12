use hydra_server::routes::{MessageRoutes, ProfileRoutes, TaskRoutes};

// ─── Profile Routes ───────────────────────────────────────

#[test]
fn test_profile_get() {
    assert_eq!(ProfileRoutes::get_profile(), "/api/profile");
}

#[test]
fn test_profile_update() {
    assert_eq!(ProfileRoutes::update_profile(), "/api/profile");
}

#[test]
fn test_profile_set_name() {
    assert_eq!(ProfileRoutes::set_name(), "/api/profile/name");
}

#[test]
fn test_profile_enable_voice() {
    assert_eq!(ProfileRoutes::enable_voice(), "/api/profile/voice/enable");
}

#[test]
fn test_profile_complete_onboarding() {
    assert_eq!(
        ProfileRoutes::complete_onboarding(),
        "/api/profile/onboarding/complete"
    );
}

#[test]
fn test_profile_is_first_run() {
    assert_eq!(ProfileRoutes::is_first_run(), "/api/profile/is-first-run");
}

// ─── Message Routes ──────────────────────────────────────

#[test]
fn test_message_routes_conversations() {
    assert_eq!(MessageRoutes::list_conversations(), "/api/conversations");
    assert_eq!(MessageRoutes::create_conversation(), "/api/conversations");
    assert_eq!(MessageRoutes::get_conversation(), "/api/conversations/:id");
    assert_eq!(
        MessageRoutes::delete_conversation(),
        "/api/conversations/:id"
    );
}

#[test]
fn test_message_routes_messages() {
    assert_eq!(
        MessageRoutes::list_messages(),
        "/api/conversations/:id/messages"
    );
    assert_eq!(
        MessageRoutes::send_message(),
        "/api/conversations/:id/messages"
    );
    assert_eq!(
        MessageRoutes::get_message(),
        "/api/conversations/:id/messages/:msg_id"
    );
    assert_eq!(
        MessageRoutes::retry_message(),
        "/api/conversations/:id/retry"
    );
}

// ─── Task Routes ─────────────────────────────────────────

#[test]
fn test_task_routes_crud() {
    assert_eq!(TaskRoutes::list_tasks(), "/api/tasks");
    assert_eq!(TaskRoutes::create_task(), "/api/tasks");
    assert_eq!(TaskRoutes::get_task(), "/api/tasks/:id");
    assert_eq!(TaskRoutes::update_task(), "/api/tasks/:id");
    assert_eq!(TaskRoutes::cancel_task(), "/api/tasks/:id");
}

#[test]
fn test_task_routes_lifecycle() {
    assert_eq!(TaskRoutes::pause_task(), "/api/tasks/:id/pause");
    assert_eq!(TaskRoutes::resume_task(), "/api/tasks/:id/resume");
    assert_eq!(TaskRoutes::task_status(), "/api/tasks/:id/status");
    assert_eq!(TaskRoutes::list_subtasks(), "/api/tasks/:id/subtasks");
}

#[test]
fn test_all_routes_start_with_api() {
    let all_routes = vec![
        ProfileRoutes::get_profile(),
        ProfileRoutes::update_profile(),
        ProfileRoutes::set_name(),
        ProfileRoutes::enable_voice(),
        ProfileRoutes::complete_onboarding(),
        ProfileRoutes::get_greeting(),
        ProfileRoutes::is_first_run(),
        MessageRoutes::list_conversations(),
        MessageRoutes::create_conversation(),
        MessageRoutes::get_conversation(),
        MessageRoutes::delete_conversation(),
        MessageRoutes::list_messages(),
        MessageRoutes::send_message(),
        MessageRoutes::get_message(),
        MessageRoutes::retry_message(),
        TaskRoutes::list_tasks(),
        TaskRoutes::create_task(),
        TaskRoutes::get_task(),
        TaskRoutes::update_task(),
        TaskRoutes::cancel_task(),
        TaskRoutes::pause_task(),
        TaskRoutes::resume_task(),
        TaskRoutes::task_status(),
        TaskRoutes::list_subtasks(),
    ];

    for route in all_routes {
        assert!(
            route.starts_with("/api/"),
            "Route '{}' should start with /api/",
            route
        );
    }
}
