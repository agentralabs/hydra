// Pre-rendered overlays element: onboarding, command palette, features, receipts
// Included as `let overlays_el: Element = include!("app_rsx_overlays_el.rs");`
rsx! {
    // ══ Onboarding overlay ══
    if *show_onboarding.read() {
        div {
            class: "onboarding-overlay",
            role: "dialog",
            aria_label: "Welcome to Hydra",
            div {
                class: "onboarding-card",
                div { class: "onboarding-globe" }
                {
                    let ob = onboarding.read();
                    let view = ob.current_view();
                    rsx! {
                        h2 { class: "onboarding-title", "{view.title}" }
                        p { class: "onboarding-subtitle", "{view.subtitle}" }
                    }
                }
                match onboarding.read().step {
                    OnboardingStep::Intro => rsx! {
                        button {
                            class: "btn-primary",
                            onclick: move |_| { onboarding.write().advance(); },
                            "Continue"
                        }
                    },
                    OnboardingStep::AskName => rsx! {
                        input {
                            class: "onboarding-input",
                            placeholder: "Your name...",
                            value: "{input}",
                            oninput: move |e| input.set(e.value()),
                            onkeypress: move |e| {
                                if e.key() == Key::Enter && !input.read().trim().is_empty() {
                                    onboarding.write().set_name(input.read().trim());
                                    onboarding.write().advance();
                                    input.set(String::new());
                                }
                            }
                        }
                        button {
                            class: "btn-primary",
                            onclick: move |_| {
                                if !input.read().trim().is_empty() {
                                    onboarding.write().set_name(input.read().trim());
                                    onboarding.write().advance();
                                    input.set(String::new());
                                }
                            },
                            "Continue"
                        }
                    },
                    OnboardingStep::AskApiKey => rsx! {
                        input {
                            class: "onboarding-input",
                            placeholder: "sk-ant-api03-... or sk-...",
                            value: "{input}",
                            oninput: move |e| input.set(e.value()),
                            onkeypress: move |e| {
                                if e.key() == Key::Enter && !input.read().trim().is_empty() {
                                    onboarding.write().set_api_key(input.read().trim());
                                    onboarding.write().advance();
                                    input.set(String::new());
                                }
                            }
                        }
                        div {
                            class: "onboarding-buttons",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    if !input.read().trim().is_empty() {
                                        onboarding.write().set_api_key(input.read().trim());
                                        onboarding.write().advance();
                                        input.set(String::new());
                                    }
                                },
                                "Continue"
                            }
                            button {
                                class: "btn-secondary",
                                onclick: move |_| { onboarding.write().advance(); },
                                "Skip for now"
                            }
                        }
                    },
                    OnboardingStep::AskVoice => rsx! {
                        div {
                            class: "onboarding-buttons",
                            button {
                                class: "btn-primary",
                                onclick: move |_| { onboarding.write().enable_voice(); onboarding.write().advance(); },
                                "Yes, enable"
                            }
                            button {
                                class: "btn-secondary",
                                onclick: move |_| { onboarding.write().advance(); },
                                "Maybe later"
                            }
                        }
                    },
                    OnboardingStep::Complete => rsx! {
                        button {
                            class: "btn-primary",
                            onclick: move |_| {
                                save_current_profile();
                                show_onboarding.set(false);
                            },
                            "Get started"
                        }
                    },
                }
                // Step dots (5 steps: Intro, AskName, AskApiKey, AskVoice, Complete)
                div {
                    class: "step-dots",
                    {
                        let step = onboarding.read().step;
                        let steps = [
                            OnboardingStep::Intro,
                            OnboardingStep::AskName,
                            OnboardingStep::AskApiKey,
                            OnboardingStep::AskVoice,
                            OnboardingStep::Complete,
                        ];
                        let current_idx = steps.iter().position(|s| *s == step).unwrap_or(0);
                        let c0: &str = if current_idx == 0 { "step-dot active" } else { "step-dot done" };
                        let c1: &str = if current_idx < 1 { "step-dot" } else if current_idx == 1 { "step-dot active" } else { "step-dot done" };
                        let c2: &str = if current_idx < 2 { "step-dot" } else if current_idx == 2 { "step-dot active" } else { "step-dot done" };
                        let c3: &str = if current_idx < 3 { "step-dot" } else if current_idx == 3 { "step-dot active" } else { "step-dot done" };
                        let c4: &str = if current_idx < 4 { "step-dot" } else { "step-dot active" };
                        rsx! {
                            div { class: c0 }
                            div { class: c1 }
                            div { class: c2 }
                            div { class: c3 }
                            div { class: c4 }
                        }
                    }
                }
            }
        }
    }

    // ══ Command Palette ══
    if *show_command_palette.read() {
        div {
            class: "command-palette-overlay",
            role: "dialog",
            aria_label: "Command palette",
            onclick: move |_| show_command_palette.set(false),
            div {
                class: "command-palette",
                onclick: move |e: MouseEvent| e.stop_propagation(),
                input {
                    class: "command-palette-input",
                    placeholder: "Type a command...",
                    value: "{command_palette.read().query}",
                    autofocus: true,
                    oninput: move |e| command_palette.write().set_query(&e.value()),
                    onkeydown: move |e| {
                        match e.key() {
                            Key::ArrowUp => { e.prevent_default(); command_palette.write().select_up(); }
                            Key::ArrowDown => { e.prevent_default(); command_palette.write().select_down(); }
                            Key::Enter => {
                                e.prevent_default();
                                let id = command_palette.read().selected_command_id();
                                if let Some(cmd_id) = id {
                                    command_palette.write().record_usage(&cmd_id);
                                    show_command_palette.set(false);
                                    match cmd_id.as_str() {
                                        "toggle-sidebar" => { let c = *show_sidebar.read(); show_sidebar.set(!c); }
                                        "open-settings" => show_settings.set(true),
                                        "mode-companion" => current_mode.set("companion".into()),
                                        "mode-workspace" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                                        "mode-immersive" => current_mode.set("immersive".into()),
                                        "mode-invisible" => current_mode.set("invisible".into()),
                                        "clear-chat" => messages.write().clear(),
                                        "view-features" => show_features.set(true),
                                        "toggle-kill-switch" => {
                                            palette_approval_mgr.cancel_all();
                                            is_typing.set(false);
                                            phase.set("Idle".into());
                                            icon_state.set("idle".into());
                                            pending_approval.set(None);
                                            pending_approval_id.set(None);
                                            phase_statuses.set(vec![]);
                                            active_error.set(Some(FriendlyError {
                                                message: "Kill Switch Activated".into(),
                                                explanation: "All operations halted. Press Escape to dismiss.".into(),
                                                options: vec![],
                                                icon_state: "error".into(),
                                                can_undo: false,
                                            }));
                                        }
                                        "search-messages" => {
                                            show_search.set(true);
                                            search_query.set(String::new());
                                        }
                                        "view-receipts" => {
                                            show_receipts.set(true);
                                        }
                                        "new-session" => {
                                            // Save current session
                                            let cur_id = active_session_id.read().clone();
                                            let cur_msgs = messages.read().clone();
                                            session_store.write().insert(cur_id, cur_msgs);
                                            let task_ids: Vec<String> = sidebar.read().today_items().iter()
                                                .filter(|item| item.active)
                                                .map(|item| item.id.clone())
                                                .collect();
                                            for id in task_ids {
                                                sidebar.write().complete_task(&id);
                                            }
                                            // Create new DB conversation
                                            let _conv_id = chat_db_sig.read().new_conversation();
                                            let count = *session_counter.read() + 1;
                                            session_counter.set(count);
                                            let new_id = format!("session-{}", count);
                                            sidebar.write().add_task(&new_id, &format!("Session {}", count));
                                            active_session_id.set(new_id);
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
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Key::Escape => show_command_palette.set(false),
                            _ => {}
                        }
                    },
                }
                div {
                    class: "command-palette-results",
                    {
                        let cp = command_palette.read();
                        let filtered = cp.filtered();
                        let selected = cp.selected_index;
                        rsx! {
                            for (idx, cmd) in filtered.iter().enumerate() {
                                div {
                                    class: if idx == selected { "command-item selected" } else { "command-item" },
                                    span { class: "command-label", "{cmd.label}" }
                                    if let Some(ref shortcut) = cmd.shortcut {
                                        span { class: "command-shortcut", "{shortcut}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ══ Features overlay ══
    if *show_features.read() {
        div {
            class: "features-overlay",
            onclick: move |_| show_features.set(false),
            div {
                class: "features-card",
                onclick: move |e: MouseEvent| e.stop_propagation(),
                h2 { class: "onboarding-title", "Features & Capabilities" }
                p { class: "features-subtitle", "All Hydra systems and inventions" }

                div { class: "feature-section",
                    h3 { class: "feature-section-title", "Cognitive Inventions" }
                    div { class: "feature-grid",
                        div { class: "feature-chip active", "Dream State" }
                        div { class: "feature-chip active", "Shadow Self" }
                        div { class: "feature-chip active", "Resurrection" }
                        div { class: "feature-chip active", "Token Minimizer" }
                        div { class: "feature-chip active", "Future Echo" }
                        div { class: "feature-chip active", "Mutation Engine" }
                        div { class: "feature-chip active", "Forking" }
                    }
                }
                div { class: "feature-section",
                    h3 { class: "feature-section-title", "Agent Swarm & Federation" }
                    div { class: "feature-grid",
                        div { class: "feature-chip active", "Peer Discovery" }
                        div { class: "feature-chip active", "Task Delegation" }
                        div { class: "feature-chip active", "Skill Sharing" }
                        div { class: "feature-chip active", "Load Balancing" }
                        div { class: "feature-chip", "Multi-Instance Sync" }
                    }
                }
                div { class: "feature-section",
                    h3 { class: "feature-section-title", "Safety & Control" }
                    div { class: "feature-grid",
                        div { class: "feature-chip active", "Execution Gate" }
                        div { class: "feature-chip active", "Kill Switch" }
                        div { class: "feature-chip active", "Risk Assessment" }
                        div { class: "feature-chip active", "Boundary Enforcer" }
                        div { class: "feature-chip active", "Challenge Phrases" }
                    }
                }
                div { class: "feature-section",
                    h3 { class: "feature-section-title", "Sisters & Skills" }
                    div { class: "feature-grid",
                        div { class: "feature-chip active", "Memory Bridge" }
                        div { class: "feature-chip active", "Vision Bridge" }
                        div { class: "feature-chip active", "Codebase Bridge" }
                        div { class: "feature-chip active", "Identity Bridge" }
                        div { class: "feature-chip active", "Skill Registry" }
                    }
                }

                button {
                    class: "btn-primary overlay-close-btn",
                    onclick: move |_| show_features.set(false),
                    "Close"
                }
            }
        }
    }

    // ══ Receipts overlay ══
    if *show_receipts.read() {
        div {
            class: "overlay",
            div {
                class: "overlay-panel receipts-panel",
                h2 { class: "overlay-title", "Receipt Audit Log" }
                div {
                    class: "receipts-list",
                    if messages.read().is_empty() {
                        p { class: "receipts-empty", "No actions recorded yet. Send a message to start." }
                    }
                    for (i, (role, content, _)) in messages.read().iter().enumerate() {
                        {
                            let role_label = if role == "user" { "You" } else { "Hydra" };
                            let preview = if content.len() > 100 { format!("{}...", &content[..97]) } else { content.clone() };
                            rsx! {
                                div {
                                    class: "receipt-row",
                                    key: "{i}",
                                    span { class: "receipt-index", "#{i}" }
                                    span { class: "receipt-role", "{role_label}" }
                                    span { class: "receipt-preview", "{preview}" }
                                }
                            }
                        }
                    }
                }
                button {
                    class: "btn-primary",
                    class: "overlay-close-btn",
                    onclick: move |_| show_receipts.set(false),
                    "Close"
                }
            }
        }
    }
}
