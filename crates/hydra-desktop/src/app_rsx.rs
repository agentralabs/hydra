// Main RSX composition — wrapped in a block so include!() sees a single expression.
// Sub-elements use include!() for rsx! blocks (valid single expressions).
{
// Shared voice toggle closure — used by both companion and workspace
let toggle_voice = include!("app_voice_trigger.rs");

// Auto-listen bridge: start mic ONLY when cognitive is done AND TTS finished playing.
// peek() avoids subscribing (prevents infinite re-render loop from read+write).
if *companion_auto_listen.peek() && !*voice_listening.peek()
    && *cognitive_done.peek() && !*tts_playing.peek()
{
    companion_auto_listen.set(false);
    cognitive_done.set(false);
    toggle_voice.call(());
}

// Pre-render independent sections
let overlays_el: Element = include!("app_rsx_overlays_el.rs");
let sidebar_el: Element = include!("app_rsx_sidebar_el.rs");
let ghost_el: Element = include!("app_rsx_ghost_el.rs");
let companion_el: Element = include!("app_rsx_companion.rs");

// Lazy closures for conditional settings sections
let settings_a = include!("app_rsx_settings_a.rs");
let settings_b = include!("app_rsx_settings_b.rs");
let settings_about = include!("app_rsx_settings_about.rs");
let settings_mcp = include!("app_rsx_settings_mcp.rs");

// Chat section elements
let chat_body_el: Element = include!("app_rsx_chat_body.rs");
let chat_controls_el: Element = include!("app_rsx_chat_controls.rs");

rsx! {
    style { {CSS} }

    // Root — theme applied via CSS class (reactive, no script needed)
    div {
        class: {
            let theme = settings_theme.read().clone();
            let theme_class = match theme.as_str() {
                "light" => "app-root theme-light",
                "system" => "app-root theme-system",
                _ => "app-root",
            };
            theme_class.to_string()
        },

        {overlays_el}

        // ══ Main layout ══
        div {
            class: if *show_sidebar.read() { "app-layout with-sidebar" } else { "app-layout" },

            {sidebar_el}

            // ── Main content ──
            div {
                class: "main-content",

                // ── Topbar ──
                div {
                    class: "topbar",
                    div {
                        class: "topbar-left",
                        if !*show_sidebar.read() {
                            span { class: "topbar-brand", "Hydra" }
                        }
                        span { class: "topbar-version", "v{HYDRA_VERSION}" }
                        span { class: "topbar-sep", "\u{00B7}" }
                        span { class: "topbar-model", "{model_display}" }
                        span { class: "topbar-sep", "\u{00B7}" }
                        {
                            let mode = current_mode.read().clone();
                            let mode_label = match mode.as_str() {
                                "companion" => "Companion",
                                "workspace" => "Workspace",
                                "immersive" => "Immersive",
                                "invisible" => "Invisible",
                                other => other,
                            };
                            let is_comp = mode == "companion";
                        rsx! {
                            div {
                                class: "mode-toggle-pill",
                                button {
                                    class: if !is_comp { "mode-opt active" } else { "mode-opt" },
                                    title: "Text + Voice",
                                    onclick: move |_| { current_mode.set("workspace".into()); show_sidebar.set(true); },
                                    "\u{2338}"
                                }
                                button {
                                    class: if is_comp { "mode-opt active" } else { "mode-opt" },
                                    title: "Voice",
                                    onclick: move |_| { current_mode.set("companion".into()); show_sidebar.set(false); },
                                    "\u{266A}"
                                }
                            }
                        }
                        }
                    }
                    div {
                        class: "topbar-center",
                        // Phase dots — debug mode only
                        if *settings_debug_mode.read() {
                            {
                                let statuses = phase_statuses.read();
                                if !statuses.is_empty() {
                                    let dots = build_phase_dots(&statuses);
                                    let connectors = build_connectors(&statuses);
                                    rsx! {
                                        div {
                                            class: "phase-dots",
                                            for (idx, dot) in dots.iter().enumerate() {
                                                if idx > 0 {
                                                    { let conn = &connectors[idx - 1];
                                                      rsx! { div { class: if conn.active { "phase-connector active" } else { "phase-connector" } } } }
                                                }
                                                div { title: "{dot.label}", class: format!("phase-dot {}", dot.css_class) }
                                            }
                                        }
                                    }
                                } else { rsx! {} }
                            }
                        }
                        // Simple status label — always visible when working
                        {
                            let p = phase.read();
                            let label = match p.as_str() {
                                "Perceive" | "Think" | "Decide" | "Act" => "Working...",
                                "Learn" => "Finishing up...",
                                "Error" => "Error",
                                _ => "",
                            };
                            if !label.is_empty() {
                                rsx! { span { class: "topbar-phase-label", "{label}" } }
                            } else { rsx! {} }
                        }
                    }
                    div {
                        class: "topbar-right",
                        button {
                            class: "topbar-cmd-btn",
                            title: "Command Palette (Cmd+K)",
                            aria_label: "Open command palette",
                            onclick: move |_| { command_palette.write().reset(); show_command_palette.set(true); },
                            "\u{2318}K"
                        }
                        button {
                            class: "topbar-icon-btn",
                            title: "Toggle Sidebar (Cmd+B)",
                            aria_label: "Toggle sidebar",
                            onclick: move |_| { let c = *show_sidebar.read(); show_sidebar.set(!c); },
                            "\u{2630}"
                        }
                        button {
                            class: "topbar-icon-btn",
                            title: "Settings (Cmd+,)",
                            aria_label: "Open settings",
                            onclick: move |_| { let c = *show_settings.read(); show_settings.set(!c); },
                            "\u{2699}"
                        }
                    }
                }

                // ── Content: Settings OR Chat ──
                if *show_settings.read() {
                    div {
                        class: "settings-page",
                        // Left nav
                        div {
                            class: "settings-nav",
                            {
                                let tabs: Vec<(&str, &str, &str)> = vec![
                                    ("general", "\u{2699}", "General"),
                                    ("models", "\u{2B21}", "Models"),
                                    ("integrations", "\u{2B12}", "Integrations"),
                                    ("sisters", "\u{2726}", "Sisters"),
                                    ("voice", "\u{266A}", "Voice"),
                                    ("policies", "\u{26E8}", "Policies"),
                                    ("behavior", "\u{2699}", "Behavior"),
                                    ("advanced", "\u{2318}", "Advanced"),
                                    ("about", "\u{24D8}", "About"),
                                ];
                                let current_tab = settings_tab.read().clone();
                                rsx! {
                                    for (id, icon, label) in tabs.iter() {
                                        button {
                                            class: if current_tab == *id { "settings-nav-item active" } else { "settings-nav-item" },
                                            onclick: {
                                                let tab_id = id.to_string();
                                                move |_| settings_tab.set(tab_id.clone())
                                            },
                                            span { class: "settings-nav-icon", "{icon}" }
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                        // Right body
                        div {
                            class: "settings-body",
                            {
                                let tab = settings_tab.read().clone();
                                match tab.as_str() {
                                    "general" | "models" | "sisters" => settings_a(&tab),
                                    "integrations" => settings_mcp(),
                                    "about" => settings_about(),
                                    _ => settings_b(&tab),
                                }
                            }
                            // Save button
                            div { class: "settings-save-area",
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        save_current_profile();
                                        let mode = settings_default_mode.read().clone();
                                        current_mode.set(mode.clone());
                                        show_sidebar.set(mode == "workspace");
                                        show_settings.set(false);
                                    },
                                    "Save & Close"
                                }
                                button {
                                    class: "btn-secondary",
                                    onclick: move |_| show_settings.set(false),
                                    "Cancel"
                                }
                            }
                        }
                    }
                } else if *current_mode.read() == "companion" {
                    {companion_el}
                } else if *current_mode.read() == "invisible" {
                    div {
                        class: "invisible-mode",
                        {
                            let p = phase.read();
                            let has_approval = pending_approval.read().is_some();
                            let voice_on = *settings_voice.read();
                            let globe_state = derive_globe_state(&p, has_approval, voice_on, false);
                            let params = globe_params(globe_state);
                            let svg_html = globe_svg(&params, GlobeSize::Compact.pixels());
                            rsx! { div { class: "voice-globe", dangerous_inner_html: svg_html } }
                        }
                        p { class: "invisible-hint", "Say \"Hey Hydra\" or \u{2318}1 for companion" }
                    }
                } else {
                    // Workspace / chat view
                    div {
                        class: "chat-container",

                        // ── Dynamic Companion Island ──
                        // Persistent globe indicator — click to enter companion.
                        // Expands when Hydra is processing or voice is active.
                        div {
                            class: "companion-island",
                            onclick: move |_| { current_mode.set("companion".into()); },
                            {
                                let p = phase.read();
                                let has_approval = pending_approval.read().is_some();
                                let voice_on = *settings_voice.read();
                                let listening = *voice_listening.read();
                                let globe_state = derive_globe_state(&p, has_approval, voice_on, listening);
                                let params = globe_params(globe_state);
                                let svg_html = globe_svg(&params, GlobeSize::TopBar.pixels());
                                let is_active = !matches!(p.as_str(), "Idle" | "" | "Done");
                                let island_class = if is_active || listening {
                                    "island-inner expanded"
                                } else {
                                    "island-inner"
                                };
                                let status_text = match p.as_str() {
                                    "Perceive" | "Think" => "Thinking...",
                                    "Decide" => "Deciding...",
                                    "Act" => "Working...",
                                    "Learn" => "Learning...",
                                    "Error" => "Error",
                                    _ => if listening { "Listening..." } else { "" },
                                };
                                rsx! {
                                    div {
                                        class: island_class,
                                        div {
                                            class: format!("island-globe {}", params.animation),
                                            dangerous_inner_html: svg_html,
                                        }
                                        if !status_text.is_empty() {
                                            span { class: "island-status", "{status_text}" }
                                        }
                                        button {
                                            class: if listening { "island-mic active" } else { "island-mic" },
                                            title: "Voice",
                                            onclick: move |e| {
                                                e.stop_propagation();
                                                toggle_voice.call(());
                                            },
                                            dangerous_inner_html: r#"<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10a7 7 0 0 0 14 0"/></svg>"#,
                                        }
                                    }
                                }
                            }
                        }

                        {chat_body_el}
                        {chat_controls_el}
                    }
                }
            }
        }

        {ghost_el}
    }
}
}
