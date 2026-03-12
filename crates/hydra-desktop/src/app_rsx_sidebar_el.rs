// Pre-rendered sidebar element
// Included as `let sidebar_el: Element = include!("app_rsx_sidebar_el.rs");`
rsx! {
    // ── Sidebar ──
    if *show_sidebar.read() {
        div {
            class: "sidebar",
            div {
                class: "sidebar-header",
                span { class: "sidebar-brand", "Hydra" }
                button {
                    class: "sidebar-new-btn",
                    title: "New Session (Cmd+N)",
                    onclick: move |_| {
                        // Save current session messages
                        let cur_id = active_session_id.read().clone();
                        let cur_msgs = messages.read().clone();
                        session_store.write().insert(cur_id, cur_msgs);
                        // Complete active sidebar tasks
                        let task_ids: Vec<String> = sidebar.read().today_items().iter()
                            .filter(|item| item.active)
                            .map(|item| item.id.clone())
                            .collect();
                        for id in task_ids {
                            sidebar.write().complete_task(&id);
                        }
                        // Create new DB conversation
                        let _conv_id = chat_db_sig.read().new_conversation();
                        // Create new session
                        let count = *session_counter.read() + 1;
                        session_counter.set(count);
                        let new_id = format!("session-{}", count);
                        sidebar.write().add_task(&new_id, &format!("Session {}", count));
                        active_session_id.set(new_id);
                        // Reset state for fresh session
                        messages.write().clear();
                        phase.set("Idle".into());
                        icon_state.set("idle".into());
                        is_typing.set(false);
                        connected.set(false);
                        input.set(String::new());
                        plan_panel.write().steps.clear();
                        timeline_panel.write().clear();
                        evidence_panel.write().clear();
                        pending_approval.set(None);
                        celebration.set(None);
                        active_error.set(None);
                        phase_statuses.set(vec![]);
                        challenge_input.set(String::new());
                        new_session_flash.set(true);
                    },
                    "+"
                }
            }
            div {
                class: "sidebar-sessions",
                div {
                    class: "sidebar-section-title",
                    onclick: move |_| sidebar.write().toggle_section(0),
                    span { "Today" }
                    {
                        let ch = if sidebar.read().sections.first().map_or(false, |s| s.collapsed) { "\u{25B8}" } else { "\u{25BE}" };
                        rsx! { span { class: "sidebar-chevron", "{ch}" } }
                    }
                }
                if !sidebar.read().sections.first().map_or(true, |s| s.collapsed) {
                    for item in sidebar.read().today_items().iter() {
                        {
                            let item_id = item.id.clone();
                            let is_done = item.icon == "\u{2713}";
                            let label = item.label.clone();
                            let current_session = active_session_id.read().clone();
                            let is_current = item_id == current_session;
                            let item_class = if is_current { "sidebar-item active" } else { "sidebar-item" };
                            let dot_class = if is_current { "sidebar-dot pulse" } else if is_done { "sidebar-dot done" } else { "sidebar-dot" };
                            rsx! {
                                div {
                                    class: item_class.to_string(),
                                    onclick: {
                                        let switch_id = item_id.clone();
                                        move |_| {
                                            let cur_id = active_session_id.read().clone();
                                            if switch_id == cur_id { return; }
                                            // Save current session
                                            let cur_msgs = messages.read().clone();
                                            session_store.write().insert(cur_id.clone(), cur_msgs);
                                            // Deactivate old, activate new in sidebar
                                            sidebar.write().complete_task(&cur_id);
                                            // Switch DB conversation (switch_id maps to a conversation)
                                            let db_msgs = chat_db_sig.read().switch_conversation(&switch_id);
                                            // Load target session messages (prefer DB, fall back to in-memory)
                                            let stored = if !db_msgs.is_empty() {
                                                db_msgs
                                            } else {
                                                session_store.read().get(&switch_id).cloned().unwrap_or_default()
                                            };
                                            *messages.write() = stored.clone();
                                            connected.set(!stored.is_empty());
                                            active_session_id.set(switch_id.clone());
                                            // Reset transient state
                                            phase.set("Idle".into());
                                            icon_state.set("idle".into());
                                            is_typing.set(false);
                                            input.set(String::new());
                                            pending_approval.set(None);
                                            new_session_flash.set(false);
                                        }
                                    },
                                    span { class: dot_class.to_string() }
                                    span { class: "sidebar-item-label", "{label}" }
                                    if is_current {
                                        span { class: "sidebar-item-badge", "Active" }
                                    }
                                    // Archive & delete buttons (not on active session)
                                    if !is_current {
                                        div {
                                            class: "sidebar-item-actions",
                                            button {
                                                class: "sidebar-action-btn",
                                                title: "Archive",
                                                onclick: {
                                                    let archive_id = item_id.clone();
                                                    move |e: Event<MouseData>| {
                                                        e.stop_propagation();
                                                        sidebar.write().archive_task(&archive_id);
                                                    }
                                                },
                                                span { class: "action-icon icon-archive" }
                                            }
                                            button {
                                                class: "sidebar-action-btn delete",
                                                title: "Delete",
                                                onclick: {
                                                    let delete_id = item_id.clone();
                                                    move |e: Event<MouseData>| {
                                                        e.stop_propagation();
                                                        sidebar.write().remove_task(&delete_id);
                                                        session_store.write().remove(&delete_id);
                                                    }
                                                },
                                                span { class: "action-icon icon-delete" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // History section
                if !sidebar.read().history_items().is_empty() {
                    div {
                        class: "sidebar-section-title",
                        onclick: move |_| sidebar.write().toggle_section(1),
                        span { "History" }
                        {
                            let ch = if sidebar.read().sections.get(1).map_or(true, |s| s.collapsed) { "\u{25B8}" } else { "\u{25BE}" };
                            rsx! { span { class: "sidebar-chevron", "{ch}" } }
                        }
                    }
                    if !sidebar.read().sections.get(1).map_or(true, |s| s.collapsed) {
                        for item in sidebar.read().history_items().iter() {
                            {
                                let item_id = item.id.clone();
                                let label = item.label.clone();
                                rsx! {
                                    div {
                                        class: "sidebar-item history",
                                        onclick: {
                                            let switch_id = item_id.clone();
                                            move |_| {
                                                // Save current, switch to archived
                                                let cur_id = active_session_id.read().clone();
                                                let cur_msgs = messages.read().clone();
                                                session_store.write().insert(cur_id.clone(), cur_msgs);
                                                sidebar.write().complete_task(&cur_id);
                                                // Switch DB conversation
                                                let db_msgs = chat_db_sig.read().switch_conversation(&switch_id);
                                                let stored = if !db_msgs.is_empty() {
                                                    db_msgs
                                                } else {
                                                    session_store.read().get(&switch_id).cloned().unwrap_or_default()
                                                };
                                                *messages.write() = stored.clone();
                                                connected.set(!stored.is_empty());
                                                active_session_id.set(switch_id.clone());
                                                phase.set("Idle".into());
                                                icon_state.set("idle".into());
                                                is_typing.set(false);
                                                input.set(String::new());
                                                pending_approval.set(None);
                                                new_session_flash.set(false);
                                            }
                                        },
                                        span { class: "sidebar-dot done" }
                                        span { class: "sidebar-item-label", "{label}" }
                                        div {
                                            class: "sidebar-item-actions",
                                            button {
                                                class: "sidebar-action-btn delete",
                                                title: "Delete",
                                                onclick: {
                                                    let del_id = item_id.clone();
                                                    move |e: Event<MouseData>| {
                                                        e.stop_propagation();
                                                        sidebar.write().remove_task(&del_id);
                                                        session_store.write().remove(&del_id);
                                                    }
                                                },
                                                span { class: "action-icon icon-delete" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div {
                class: "sidebar-footer",
                button {
                    class: "sidebar-settings-btn",
                    onclick: move |_| {
                        // Launch TUI in a new terminal window
                        let project_root = env!("CARGO_MANIFEST_DIR")
                            .trim_end_matches("/crates/hydra-desktop");

                        #[cfg(target_os = "macos")]
                        {
                            // Use open -a Terminal with a shell script
                            let script = format!(
                                "cd {} && cargo run -q --bin hydra-cli",
                                project_root
                            );
                            let _ = std::process::Command::new("osascript")
                                .arg("-e")
                                .arg(format!(
                                    "tell application \"Terminal\"\n\
                                        set hydraRunning to false\n\
                                        repeat with w in windows\n\
                                            repeat with t in tabs of w\n\
                                                if processes of t contains \"hydra-cli\" then\n\
                                                    set hydraRunning to true\n\
                                                    set selected tab of w to t\n\
                                                    set frontmost of w to true\n\
                                                end if\n\
                                            end repeat\n\
                                        end repeat\n\
                                        if not hydraRunning then\n\
                                            do script \"{}\"\n\
                                        end if\n\
                                        activate\n\
                                    end tell",
                                    script
                                ))
                                .spawn();
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = std::process::Command::new("sh")
                                .args([
                                    "-c",
                                    &format!(
                                        "x-terminal-emulator -e 'cd {} && cargo run -q --bin hydra-cli' &",
                                        project_root
                                    ),
                                ])
                                .spawn();
                        }
                    },
                    title: "Open Terminal (TUI)",
                    ">"  // terminal icon
                }
                div {
                    class: "sidebar-status",
                    div { class: if *connected.read() { "status-dot connected" } else { "status-dot" } }
                    span { class: "sidebar-status-text", if *connected.read() { "Connected" } else { "Ready" } }
                }
            }
        }
    }
}
