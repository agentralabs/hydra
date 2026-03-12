use super::*;

// ── App Creation & Defaults ──

#[test]
fn app_creation() {
    let app = App::new();
    assert_eq!(app.sisters.len(), 14);
    assert_eq!(app.total_sisters, 14);
    assert!(!app.should_quit);
    assert_eq!(app.input_mode, InputMode::Insert);
    assert_eq!(app.boot_state, BootState::Booting);
    assert_eq!(app.permission_mode, PermissionMode::Normal);
    assert!(!app.sidebar_visible);
    assert!(app.cognitive_rx.is_none());
    assert!(app.pending_approval.is_none());
    assert!(!app.is_thinking);
}

// ── Permission Mode ──

#[test]
fn permission_mode_cycling() {
    let mut mode = PermissionMode::Normal;
    mode = mode.next();
    assert_eq!(mode, PermissionMode::AutoAccept);
    assert_eq!(mode.label(), "[Auto-Accept]");
    mode = mode.next();
    assert_eq!(mode, PermissionMode::Plan);
    assert_eq!(mode.label(), "[Plan]");
    mode = mode.next();
    assert_eq!(mode, PermissionMode::Normal);
    assert_eq!(mode.label(), "");
}

#[test]
fn permission_mode_in_app() {
    let mut app = App::new();
    app.permission_mode = app.permission_mode.next();
    assert_eq!(app.permission_mode, PermissionMode::AutoAccept);
    app.permission_mode = app.permission_mode.next();
    assert_eq!(app.permission_mode, PermissionMode::Plan);
    app.permission_mode = app.permission_mode.next();
    assert_eq!(app.permission_mode, PermissionMode::Normal);
}

// ── History & Scroll ──

#[test]
fn history_navigation() {
    let mut app = App::new();
    app.history.push("first".to_string());
    app.history.push("second".to_string());
    app.history_prev();
    assert_eq!(app.input, "second");
    app.history_prev();
    assert_eq!(app.input, "first");
    app.history_next();
    assert_eq!(app.input, "second");
}

#[test]
fn scroll_bounds() {
    let mut app = App::new();
    app.scroll_down();
    assert_eq!(app.scroll_offset, 0);
    app.scroll_up();
    assert!(app.scroll_offset > 0);
    app.scroll_to_bottom();
    assert_eq!(app.scroll_offset, 0);
    assert!(app.is_at_bottom());
}

#[test]
fn page_up_down() {
    let mut app = App::new();
    app.page_up();
    assert_eq!(app.scroll_offset, 20);
    app.page_down();
    assert_eq!(app.scroll_offset, 0);
}

// ── Sidebar ──

#[test]
fn sidebar_toggle() {
    let mut app = App::new();
    assert!(!app.sidebar_visible);
    app.sidebar_visible = !app.sidebar_visible;
    assert!(app.sidebar_visible);
}

// ── Approval ──

#[test]
fn approval_approve_and_deny() {
    let mut app = App::new();
    app.pending_approval = Some(PendingApproval {
        approval_id: Some("t-1".into()), risk_level: "high".into(),
        action: "rm -rf /tmp/test".into(), description: "Delete".into(),
    });
    app.submit_input("y");
    assert!(app.pending_approval.is_none());
    assert!(app.messages.iter().any(|m| m.content.contains("Approved")));
    app.pending_approval = Some(PendingApproval {
        approval_id: None, risk_level: "medium".into(),
        action: "delete file".into(), description: "Delete".into(),
    });
    app.submit_input("n");
    assert!(app.pending_approval.is_none());
    assert!(app.messages.iter().any(|m| m.content.contains("Denied")));
}

// ── Slash Commands ──

#[test]
fn slash_cmd_clear() {
    let mut app = App::new();
    app.messages.push(Message {
        role: MessageRole::User,
        content: "test".to_string(),
        timestamp: "00:00".to_string(),
        phase: None,
    });
    app.submit_input("/clear");
    assert!(app.messages.is_empty());
}

#[test]
fn slash_cmd_exit() {
    let mut app = App::new();
    app.submit_input("/exit");
    assert!(app.should_quit);
}

#[test]
fn slash_cmd_version() {
    let mut app = App::new();
    app.submit_input("/version");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Hydra v"));
}

#[test]
fn slash_cmd_status() {
    let mut app = App::new();
    app.submit_input("/status");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Model"));
    assert!(last.content.contains("Sisters"));
}

#[test]
fn slash_cmd_sisters_no_box_borders() {
    let mut app = App::new();
    app.submit_input("/sisters");
    let last = app.messages.last().unwrap();
    assert!(!last.content.contains('┌'));
    assert!(!last.content.contains('┘'));
    assert!(last.content.contains("Memory"));
}

#[test]
fn slash_cmd_context() {
    let mut app = App::new();
    app.submit_input("/context");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Context Usage"));
}

#[test]
fn slash_cmd_cost() {
    let mut app = App::new();
    app.submit_input("/cost");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Session Cost"));
}

#[test]
fn slash_cmd_env() {
    let mut app = App::new();
    app.submit_input("/env");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Environment Profile"));
}

#[test]
fn slash_cmd_doctor() {
    let mut app = App::new();
    app.submit_input("/doctor");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Doctor"));
}

#[test]
fn slash_cmd_dream() {
    let mut app = App::new();
    app.submit_input("/dream");
    let last = app.messages.last().unwrap();
    assert!(last.content.to_lowercase().contains("dream"));
}

#[test]
fn slash_cmd_autonomy() {
    let mut app = App::new();
    app.submit_input("/autonomy 3");
    assert_eq!(app.trust_level, "Level 3");
}

#[test]
fn slash_cmd_help() {
    let mut app = App::new();
    app.submit_input("/help");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Hydra Commands"));
    assert!(last.content.contains("Shift+Tab"));
}

#[test]
fn slash_cmd_rewind() {
    let mut app = App::new();
    for (role, text) in [(MessageRole::System, "boot"), (MessageRole::User, "hello"), (MessageRole::Hydra, "hi")] {
        app.messages.push(Message { role, content: text.to_string(), timestamp: "0".to_string(), phase: None });
    }
    app.submit_input("/rewind");
    assert!(app.messages.iter().any(|m| m.content.contains("Rewound")));
}

#[test]
fn slash_cmd_fork() {
    let mut app = App::new();
    app.submit_input("/fork");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("forked"));
}

#[test]
fn slash_cmd_rename() {
    let mut app = App::new();
    app.submit_input("/rename auth-refactor");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("auth-refactor"));
}

#[test]
fn slash_cmd_unknown() {
    let mut app = App::new();
    app.submit_input("/nonexistent");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Unknown command"));
}

#[test]
fn slash_cmd_compact_with_focus() {
    let mut app = App::new();
    for i in 0..25 {
        app.messages.push(Message { role: MessageRole::User, content: format!("m{}", i), timestamp: "0".into(), phase: None });
    }
    app.submit_input("/compact retain the auth flow");
    assert!(app.messages[0].content.contains("Compacted"));
    assert!(app.messages[0].content.contains("retain the auth flow"));
}

// ── New Spec Commands (CC Parity §5.2-5.7) ──

#[test]
fn slash_cmd_usage() {
    let mut app = App::new();
    app.submit_input("/usage");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Usage"));
    assert!(last.content.contains("tokens"));
}

#[test]
fn slash_cmd_fast() {
    let mut app = App::new();
    app.submit_input("/fast");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("Fast Mode"));
}

#[test]
fn slash_cmd_todos() {
    let mut app = App::new();
    app.messages.push(Message {
        role: MessageRole::Hydra,
        content: "TODO: fix the auth bug".to_string(),
        timestamp: "00:00".to_string(),
        phase: None,
    });
    app.submit_input("/todos");
    let last = app.messages.last().unwrap();
    assert!(last.content.contains("TODO"));
    assert!(last.content.contains("auth bug"));
}

#[test]
fn slash_cmd_integration_commands() {
    let mut app = App::new();
    app.submit_input("/mcp");
    assert!(app.messages.last().unwrap().content.contains("MCP"));
    app.submit_input("/agents");
    assert!(app.messages.last().unwrap().content.contains("Subagents"));
    app.submit_input("/skills");
    assert!(app.messages.last().unwrap().content.contains("Skills"));
    app.submit_input("/bashes");
    assert!(app.messages.last().unwrap().content.contains("Background"));
    app.submit_input("/tasks");
    assert!(app.messages.last().unwrap().content.contains("Tasks"));
}

#[test]
fn slash_cmd_plan_enters_mode() {
    let mut app = App::new();
    app.submit_input("/plan");
    assert_eq!(app.permission_mode, PermissionMode::Plan);
}

#[test]
fn slash_cmd_sister_detail() {
    let mut app = App::new();
    app.submit_input("/sister Memory");
    assert!(app.messages.last().unwrap().content.contains("Memory"));
    app.submit_input("/sister Nonexistent");
    assert!(app.messages.last().unwrap().content.contains("not found"));
}

#[test]
fn slash_cmd_config_integration() {
    let mut app = App::new();
    app.submit_input("/diagnostics");
    assert!(app.messages.last().unwrap().content.contains("Diagnostics"));
    app.submit_input("/keybindings");
    assert!(app.messages.last().unwrap().content.contains("Keybindings"));
    app.submit_input("/hooks");
    assert!(app.messages.last().unwrap().content.contains("Hook"));
    app.submit_input("/commands");
    assert!(app.messages.last().unwrap().content.contains("All Commands"));
}

#[test]
fn challenge_phrase_gate() {
    let mut app = App::new();
    app.challenge_phrase = Some("I understand the consequences".to_string());
    app.challenge_action = Some("delete database".to_string());
    app.submit_input("I understand the consequences");
    assert!(app.challenge_phrase.is_none());
    assert!(app.messages.iter().any(|m| m.content.contains("Challenge accepted")));
}

#[test]
fn challenge_phrase_rejected() {
    let mut app = App::new();
    app.challenge_phrase = Some("I understand the consequences".to_string());
    app.challenge_action = Some("delete database".to_string());
    app.submit_input("wrong phrase");
    assert!(app.challenge_phrase.is_none());
    assert!(app.messages.iter().any(|m| m.content.contains("cancelled")));
}

// ── Command Dropdown ──

#[test]
fn command_dropdown_filter_and_nav() {
    use crate::tui::commands::CommandDropdown;
    let mut dd = CommandDropdown::default();
    dd.update_filter("/he");
    assert!(dd.visible);
    dd.update_filter("/nonexistent_cmd");
    assert!(!dd.visible);
    dd.update_filter("/");
    assert!(dd.visible);
    assert_eq!(dd.selected, 0);
    dd.select_next();
    assert_eq!(dd.selected, 1);
    dd.select_prev();
    assert_eq!(dd.selected, 0);
}

// ── Input Syntax (§4.2) ──
#[test]
fn input_hash_note() {
    let mut app = App::new();
    app.submit_input("#remember this");
    assert!(app.messages.iter().any(|m| m.content.contains("Noted")));
}

#[test]
fn input_bang_command() {
    let mut app = App::new();
    app.submit_input("!ls");
    assert!(app.messages.iter().any(|m| m.content.contains("!ls")));
}

#[test]
fn double_esc_field_exists() {
    let app = App::new();
    assert_eq!(app.last_esc_tick, 0); // field exists for Esc+Esc detection
}
