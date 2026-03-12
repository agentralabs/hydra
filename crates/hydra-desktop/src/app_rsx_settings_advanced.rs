// Advanced settings tab — server config, logging, shortcuts, data management
// Included as `include!("app_rsx_settings_advanced.rs")`
rsx! {
    h2 { class: "settings-title", "Advanced" }
    // ── Server ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Server" }
        div { class: "settings-row",
            span { class: "settings-label", "Port" }
            div { class: "settings-input-row",
                input {
                    class: "settings-input-sm",
                    r#type: "number",
                    value: "{settings_server_port}",
                    oninput: move |e| settings_server_port.set(e.value()),
                }
                span { class: "settings-desc", "http://127.0.0.1:{settings_server_port}" }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Endpoints" }
            span { class: "settings-desc", "/events (SSE) \u{00B7} /rpc (JSON-RPC) \u{00B7} /health" }
        }
    }
    // ── Logging ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Logging" }
        div { class: "settings-row",
            span { class: "settings-label", "Log level" }
            div { class: "segmented-control",
                { let opts = ["error", "warn", "info", "debug", "trace"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_log_level.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_log_level.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Debug mode" }
                span { class: "settings-desc", "Show phase timing, sister dispatch, and token usage in conversation" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_debug_mode.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_debug_mode.read(); settings_debug_mode.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_debug_mode.read() { "On" } else { "Off" };
                  rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Log file" }
            div { class: "settings-path-action",
                span { class: "settings-desc", "~/.hydra/hydra-desktop.log" }
                button {
                    class: "btn-mini btn-mini-secondary",
                    onclick: move |_| {
                        let log = format!("{}/.hydra/hydra-desktop.log", crate::platform::home_dir());
                        crate::platform::open_log_viewer(&log);
                    },
                    "View Logs"
                }
            }
        }
    }
    // ── Privacy ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Privacy" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Anonymous usage analytics" }
                span { class: "settings-desc", "Help improve Hydra by sending anonymous crash reports. No conversation data." }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_telemetry.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_telemetry.read(); settings_telemetry.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_telemetry.read() { "On" } else { "Off" };
                  rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
    }
    // ── Data Management ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Data Management" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Clear conversation history" }
                span { class: "settings-desc", "Remove all messages from the database. Sister data is not affected." }
            }
            button {
                class: "btn-mini",
                onclick: move |_| {
                    let home = crate::platform::home_dir();
                    let chat_db = format!("{}/.hydra/chat.db", home);
                    let _ = std::fs::remove_file(&chat_db);
                },
                "Clear History"
            }
        }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Export settings" }
                span { class: "settings-desc", "Save current configuration as JSON for backup or transfer" }
            }
            button {
                class: "btn-mini btn-mini-secondary",
                onclick: move |_| {
                    let home = crate::platform::home_dir();
                    let src = format!("{}/.hydra/profile.json", home);
                    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
                    let dst = format!("{}/Downloads/hydra-settings-{}.json", home, date);
                    let _ = std::fs::copy(&src, &dst);
                },
                "Export JSON"
            }
        }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Open config directory" }
                span { class: "settings-desc", "~/.hydra/ \u{2014} config, profile, database, sessions, receipts" }
            }
            button {
                class: "btn-mini btn-mini-secondary",
                onclick: move |_| {
                    crate::platform::open_path(&format!("{}/.hydra", crate::platform::home_dir()));
                },
                "Open in Finder"
            }
        }
    }
    // ── Keyboard Shortcuts ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Keyboard Shortcuts" }
        div { class: "advanced-shortcuts",
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}K" }
                span { class: "shortcut-desc", "Command palette" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}B" }
                span { class: "shortcut-desc", "Toggle sidebar" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}N" }
                span { class: "shortcut-desc", "New session" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}F" }
                span { class: "shortcut-desc", "Search messages" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}Z" }
                span { class: "shortcut-desc", "Undo last action" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}\u{21E7}K" }
                span { class: "shortcut-desc", "Kill switch \u{2014} emergency halt" }
            }
            div { class: "shortcut-row",
                span { class: "shortcut-key", "\u{2318}1-4" }
                span { class: "shortcut-desc", "Switch modes" }
            }
        }
    }
    // ── Engine Info ──
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Engine" }
        div { class: "settings-row",
            span { class: "settings-label", "Cognitive Loop" }
            span { class: "settings-desc", "5-phase: Perceive \u{2192} Think \u{2192} Decide \u{2192} Act \u{2192} Learn" }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Execution Gate" }
            span { class: "settings-desc", "6-layer security stack with receipt ledger" }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Federation" }
            span { class: "settings-desc", "Peer discovery, skill sharing, task delegation" }
        }
    }
}
