// Pre-rendered chat body element: workspace panels, search, messages
// Included as `let chat_body_el: Element = include!("app_rsx_chat_body.rs");`
rsx! {
    // Minimal plan indicator — shows only when there's an active plan (no debug panels)
    if (*current_mode.read() == "workspace" || *current_mode.read() == "immersive") && plan_panel.read().steps.len() > 1 && !*settings_debug_mode.read() {
        {
            let pp = plan_panel.read();
            let completed = pp.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
            let total = pp.steps.len();
            let pct = pp.progress_percent();
            rsx! {
                div { class: "plan-inline",
                    span { class: "plan-inline-label", "{pp.goal}" }
                    div { class: "plan-inline-bar",
                        div { class: "progress-fill", style: format!("width: {}%", pct) }
                    }
                    span { class: "plan-inline-stat", "{completed}/{total}" }
                }
            }
        }
    }

    // Debug panels — only visible with debug mode ON (Settings > Advanced)
    if *settings_debug_mode.read() && plan_panel.read().steps.len() > 1 {
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
                    }
                }
            }
            // Timeline (debug only)
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
                } else { rsx! {} }
            }
            // Evidence (debug only)
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
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else { rsx! {} }
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
                let count_text = if count == 0 { "No results".to_string() } else { format!("{count} found") };
                let count_class = if count == 0 { "search-bar-count no-results" } else { "search-bar-count" };
                rsx! {
                    if !q.is_empty() {
                        span {
                            class: count_class,
                            "{count_text}"
                        }
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
                p { class: "welcome-subtitle", "{model_display} \u{00B7} v{HYDRA_VERSION}" }

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

        for (i, (role, content, css)) in messages.read().iter().enumerate() {
            {
                // Completion summary card — render as raw HTML, no markdown
                if css == "completion" {
                    rsx! {
                        div {
                            key: "{i}",
                            class: "message completion-message",
                            dangerous_inner_html: content.clone(),
                        }
                    }
                } else {
                    let sq = search_query.read().clone();
                    let is_match = if sq.is_empty() { true } else {
                        content.to_lowercase().contains(&sq.to_lowercase())
                    };
                    let msg_class = if !sq.is_empty() && is_match { "message search-hit" } else if !sq.is_empty() { "message search-dim" } else { "message" };
                    let is_user = role == "user";
                    let role_label = if is_user { "You" } else { "Hydra" };
                    let role_class = if is_user { "message-role user" } else { "message-role assistant" };
                    let html = markdown_to_html(content);
                    let copy_content = content.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                    let token_est = if !is_user { content.len() / 4 } else { 0 };
                    let token_label = if token_est >= 1000 {
                        format!("{:.1}k", token_est as f64 / 1000.0)
                    } else if token_est > 0 {
                        format!("{}", token_est)
                    } else { String::new() };
                    rsx! {
                        div {
                            key: "{i}",
                            class: msg_class.to_string(),
                            div { class: "message-header",
                                div { class: "message-role-group",
                                    span { class: if is_user { "message-avatar user" } else { "message-avatar assistant" } }
                                    span { class: role_class, "{role_label}" }
                                    if !token_label.is_empty() {
                                        span { class: "message-tokens", "{token_label}" }
                                    }
                                }
                                div { class: "message-actions",
                                    button {
                                        class: "msg-action-btn",
                                        title: "Copy message",
                                        aria_label: "Copy message to clipboard",
                                        onclick: move |_| {
                                            let js = format!("navigator.clipboard.writeText(`{}`);", copy_content);
                                            document::eval(&js);
                                        },
                                        dangerous_inner_html: r#"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"#,
                                    }
                                }
                            }
                            div {
                                class: "message-content",
                                dangerous_inner_html: html,
                            }
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

    // Auto-scroll + code block copy
    {
        let _count = messages.read().len();
        let _typing = *is_typing.read();
        rsx! { script { "requestAnimationFrame(function(){{ var el = document.getElementById('messages-container'); if(el) el.scrollTop = el.scrollHeight; }});
            document.querySelectorAll('.message-content pre').forEach(function(pre){{
                if(pre.dataset.copyBound) return;
                pre.dataset.copyBound='1';
                pre.addEventListener('click',function(e){{
                    var r=pre.getBoundingClientRect();
                    if(e.clientX>r.right-60 && e.clientY<r.top+30){{
                        var code=pre.querySelector('code');
                        if(code)navigator.clipboard.writeText(code.textContent).then(function(){{
                            var af=pre.querySelector('::after');
                            pre.style.setProperty('--copy-label','\"Copied!\"');
                            setTimeout(function(){{pre.style.removeProperty('--copy-label')}},1500);
                        }});
                    }}
                }});
            }});" } }
    }
}
