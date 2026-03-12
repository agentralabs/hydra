                    } else if *current_mode.read() == "invisible" {
                        // Invisible mode
                        div {
                            class: "invisible-mode",
                            div { class: "welcome-globe" }
                            p { class: "invisible-hint", "Say \"Hey Hydra\" or press Cmd+1 to switch modes" }
                        }
                    } else {
                        // ╔══════════════════════════════════════╗
                        // ║  CHAT VIEW                           ║
                        // ╚══════════════════════════════════════╝
                        div {
                            class: "chat-container",

                            // Workspace panels
                            if (*current_mode.read() == "workspace" || *current_mode.read() == "immersive") && plan_panel.read().steps.len() > 1 {
                                div {
                                    class: "workspace-panels",

                                    // Plan
                                    div {
                                        class: "panel",
                                        h3 { class: "panel-title", "Plan" }
                                        {
                                            let pp = plan_panel.read();
                                            rsx! {
                                                div { class: "plan-goal", "{pp.goal}" }
                                                div { class: "plan-steps",
                                                    for step in pp.steps.iter() {
                                                        div {
                                                            class: format!("plan-step {}", match step.status {
                                                                StepStatus::Completed => "completed",
                                                                StepStatus::Running => "running",
                                                                StepStatus::Failed => "failed",
                                                                StepStatus::Skipped => "skipped",
                                                                StepStatus::Pending => "pending",
                                                            }),
                                                            span { class: "step-icon", "{PlanPanel::step_icon(step.status)}" }
                                                            span { class: "step-label", "{step.label}" }
                                                        }
                                                    }
                                                }
                                                div { class: "plan-progress",
                                                    div { class: "progress-bar",
                                                        div { class: "progress-fill", style: format!("width: {}%", pp.progress_percent()) }
                                                    }
                                                    span { class: "progress-text", "{pp.progress_percent() as u32}%" }
                                                }
                                                if let Some(eta) = pp.eta_display() {
                                                    span { class: "plan-eta", "ETA: {eta}" }
                                                }
                                            }
                                        }
                                    }

                                    // Timeline
                                    {
                                        let tp = timeline_panel.read();
                                        let user_events: Vec<_> = tp.events.iter()
                                            .filter(|e| e.kind != TimelineEventKind::PhaseChange)
                                            .collect();
                                        if !user_events.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel",
                                                    h3 { class: "panel-title", "Timeline" }
                                                    div { class: "timeline-events",
                                                        for event in user_events.iter() {
                                                            div {
                                                                class: format!("timeline-event {}", TimelinePanel::event_css_class(event.kind)),
                                                                span { class: "timeline-icon", "{TimelinePanel::event_icon(event.kind)}" }
                                                                span { class: "timeline-time", "{event.timestamp}" }
                                                                span { class: "timeline-label", "{event.title}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }

                                    // Evidence
                                    {
                                        let ep = evidence_panel.read();
                                        let items: Vec<_> = ep.items.iter()
                                            .filter(|item| EvidencePanel::is_meaningful(item))
                                            .collect();
                                        if !items.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel",
                                                    h3 { class: "panel-title", "Evidence" }
                                                    div { class: "evidence-items",
                                                        for item in items.iter() {
                                                            div {
                                                                class: format!("evidence-item {}", EvidencePanel::evidence_css_class(item.kind)),
                                                                div { class: "evidence-header",
                                                                    span { class: "evidence-icon", "{EvidencePanel::evidence_icon(item.kind)}" }
                                                                    span { class: "evidence-title", "{item.title}" }
                                                                    if item.pinned {
                                                                        span { class: "evidence-pin", "Pin" }
                                                                    }
                                                                }
                                                                {
                                                                    let summary = EvidencePanel::human_summary(item);
                                                                    rsx! { p { class: "evidence-content", "{summary}" } }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }
                            }

                            // Search bar
                            if *show_search.read() {
                                div {
                                    class: "search-bar",
                                    span { class: "search-bar-icon" }
                                    input {
                                        class: "search-bar-input",
                                        placeholder: "Search messages...",
                                        value: "{search_query}",
                                        autofocus: true,
                                        oninput: move |e| search_query.set(e.value()),
                                        onkeydown: move |e| {
                                            if e.key() == Key::Escape {
                                                show_search.set(false);
                                                search_query.set(String::new());
                                            }
                                        },
                                    }
                                    {
                                        let q = search_query.read().clone();
                                        let count = if q.is_empty() { 0 } else {
                                            messages.read().iter()
                                                .filter(|(_, content, _)| content.to_lowercase().contains(&q.to_lowercase()))
                                                .count()
                                        };
                                        rsx! {
                                            if !q.is_empty() {
                                                span { class: "search-bar-count", "{count} found" }
                                            }
                                        }
                                    }
                                    button {
                                        class: "search-bar-close",
                                        onclick: move |_| {
                                            show_search.set(false);
                                            search_query.set(String::new());
                                        },
                                        "\u{2715}"
                                    }
                                }
                            }

                            // Messages
                            div {
                                class: "messages-list",
                                id: "messages-container",

                                if messages.read().is_empty() {
                                    div {
                                        class: "welcome-state",

                                        // New session flash indicator
                                        if *new_session_flash.read() {
                                            div {
                                                class: "new-session-badge",
                                                "New Session"
                                            }
                                        }

                                        // Voice globe — SVG rendered, reactive to state
                                        {
                                            let p = phase.read();
                                            let has_approval = pending_approval.read().is_some();
                                            let voice_on = *settings_voice.read();
                                            let globe_state = derive_globe_state(&p, has_approval, voice_on, false);
                                            let params = globe_params(globe_state);
                                            let mode = current_mode.read().clone();
                                            let size = match mode.as_str() {
                                                "immersive" => GlobeSize::Full,
                                                "companion" => GlobeSize::Medium,
                                                _ => GlobeSize::Medium,
                                            };
                                            let svg_html = globe_svg(&params, size.pixels());
                                            rsx! {
                                                div {
                                                    class: "voice-globe",
                                                    dangerous_inner_html: svg_html,
                                                }
                                            }
                                        }
                                        h2 { class: "welcome-title", "{greeting}" }
                                        p { class: "welcome-subtitle", "How can I help you today?" }

                                        // Quick-start suggestions
                                        div {
                                            class: "quick-starts",
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Analyze my codebase and suggest improvements".into());
                                                },
                                                span { class: "quick-start-icon icon-search" }
                                                span { class: "quick-start-text", "Analyze codebase" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Help me debug an issue".into());
                                                },
                                                span { class: "quick-start-icon icon-debug" }
                                                span { class: "quick-start-text", "Debug an issue" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Write a new feature".into());
                                                },
                                                span { class: "quick-start-icon icon-build" }
                                                span { class: "quick-start-text", "Build a feature" }
                                            }
                                            button {
                                                class: "quick-start-card",
                                                onclick: move |_| {
                                                    input.set("Explain how this project works".into());
                                                },
                                                span { class: "quick-start-icon icon-docs" }
                                                span { class: "quick-start-text", "Explain project" }
                                            }
                                        }

                                    }
                                }

                                for (i, (role, content, _css)) in messages.read().iter().enumerate() {
                                    {
                                        let sq = search_query.read().clone();
                                        let is_match = if sq.is_empty() { true } else {
                                            content.to_lowercase().contains(&sq.to_lowercase())
                                        };
                                        let msg_class = if !sq.is_empty() && is_match { "message search-hit" } else if !sq.is_empty() { "message search-dim" } else { "message" };
                                        let role_label = if role == "user" { "You" } else { "Hydra" };
                                        let html = markdown_to_html(content);
                                        rsx! {
                                            div {
                                                key: "{i}",
                                                class: msg_class.to_string(),
                                                div { class: "message-role", "{role_label}" }
                                                div {
                                                    class: "message-content",
                                                    dangerous_inner_html: html,
                                                }
                                            }
                                        }
                                    }
                                }

                                if *is_typing.read() {
                                    div {
                                        class: "typing-indicator",
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                    }
                                }
                            }

                            // Auto-scroll
                            {
                                let _count = messages.read().len();
                                let _typing = *is_typing.read();
                                rsx! { script { "requestAnimationFrame(function(){{ var el = document.getElementById('messages-container'); if(el) el.scrollTop = el.scrollHeight; }})" } }
                            }
