// Main RSX composition — wrapped in a block so include!() sees a single expression.
// Sub-elements use include!() for rsx! blocks (valid single expressions).
{
// Pre-render independent sections
let overlays_el: Element = include!("app_rsx_overlays_el.rs");
let sidebar_el: Element = include!("app_rsx_sidebar_el.rs");
let ghost_el: Element = include!("app_rsx_ghost_el.rs");

// Lazy closures for conditional settings sections
let settings_a = include!("app_rsx_settings_a.rs");
let settings_b = include!("app_rsx_settings_b.rs");

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
                        span { class: "topbar-mode", "{current_mode}" }
                    }
                    div {
                        class: "topbar-center",
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
                                                {
                                                    let conn = &connectors[idx - 1];
                                                    rsx! {
                                                        div { class: if conn.active { "phase-connector active" } else { "phase-connector" } }
                                                    }
                                                }
                                            }
                                            div {
                                                title: "{dot.label}",
                                                class: format!("phase-dot {}", dot.css_class),
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                        {
                            let p = phase.read();
                            let label = match p.as_str() {
                                "Perceive" | "Think" | "Decide" | "Act" => "Working...",
                                "Learn" => "Finishing up...",
                                "Done" => "Done",
                                "Error" => "Error",
                                "Idle" => "",
                                other => other,
                            };
                            if !label.is_empty() {
                                rsx! { span { class: "topbar-phase-label", "{label}" } }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                    div {
                        class: "topbar-right",
                        button {
                            class: "topbar-cmd-btn",
                            title: "Command Palette (Cmd+K)",
                            onclick: move |_| { command_palette.write().reset(); show_command_palette.set(true); },
                            "\u{2318}K"
                        }
                        button {
                            class: "topbar-icon-btn",
                            title: "Toggle Sidebar (Cmd+B)",
                            onclick: move |_| { let c = *show_sidebar.read(); show_sidebar.set(!c); },
                            "\u{2630}"
                        }
                        button {
                            class: "topbar-icon-btn",
                            title: "Settings (Cmd+,)",
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
                                    ("sisters", "\u{2726}", "Sisters"),
                                    ("voice", "\u{266A}", "Voice"),
                                    ("policies", "\u{26E8}", "Policies"),
                                    ("behavior", "\u{2699}", "Behavior"),
                                    ("advanced", "\u{2318}", "Advanced"),
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
                } else if *current_mode.read() == "invisible" {
                    // Invisible mode
                    div {
                        class: "invisible-mode",
                        div { class: "welcome-globe" }
                        p { class: "invisible-hint", "Say \"Hey Hydra\" or press Cmd+1 to switch modes" }
                    }
                } else {
                    // Chat view
                    div {
                        class: "chat-container",
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
