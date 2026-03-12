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
                        // ╔══════════════════════════════════════╗
                        // ║  SETTINGS PAGE — Claude Desktop      ║
                        // ╚══════════════════════════════════════╝
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
                                match settings_tab.read().as_str() {
                                    "general" => rsx! {
                                        h2 { class: "settings-title", "General" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Appearance" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Theme" }
                                                div { class: "segmented-control",
                                                    button {
                                                        class: if *settings_theme.read() == "dark" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("dark".into()),
                                                        "Dark"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "light" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("light".into()),
                                                        "Light"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "system" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_theme.set("system".into()),
                                                        "System"
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Default Mode" }
                                                div { class: "segmented-control",
                                                    button {
                                                        class: if *settings_default_mode.read() == "companion" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_default_mode.set("companion".into()),
                                                        "Companion"
                                                    }
                                                    button {
                                                        class: if *settings_default_mode.read() == "workspace" { "segment active" } else { "segment" },
                                                        onclick: move |_| settings_default_mode.set("workspace".into()),
                                                        "Workspace"
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "models" => rsx! {
                                        h2 { class: "settings-title", "Models" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Anthropic" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("claude-sonnet-4-6", "Sonnet 4.6"), ("claude-opus-4-6", "Opus 4.6"), ("claude-haiku-4-5", "Haiku 4.5")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "model-card active" } else { "model-card" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "Anthropic API Key (sk-ant-...)",
                                                    value: "{settings_anthropic_key}",
                                                    oninput: move |e| settings_anthropic_key.set(e.value()),
                                                }
                                                if !settings_anthropic_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                            // ── OR: Sign in with Anthropic (OAuth / Claude Max subscription) ──
                                            div { class: "oauth-section", style: "margin-top: 12px; padding: 12px; border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; background: rgba(255,255,255,0.03);",
                                                {
                                                    let (status, email, tier) = oauth_status.read().clone();
                                                    if status == "authenticated" {
                                                        rsx! {
                                                            div { style: "display: flex; align-items: center; gap: 8px;",
                                                                span { style: "color: #4ade80; font-size: 14px;", "\u{2713} Signed in via Anthropic" }
                                                                if !email.is_empty() {
                                                                    span { style: "color: rgba(255,255,255,0.5); font-size: 12px;", "({email})" }
                                                                }
                                                                if !tier.is_empty() {
                                                                    span { style: "color: #a78bfa; font-size: 12px; font-weight: 600;", "{tier}" }
                                                                }
                                                            }
                                                            button {
                                                                class: "btn-secondary",
                                                                style: "margin-top: 8px; font-size: 12px; padding: 4px 12px;",
                                                                onclick: move |_| {
                                                                    let mut oauth = AnthropicOAuth::new();
                                                                    oauth.logout();
                                                                    oauth_status.set(("not_authenticated".to_string(), String::new(), String::new()));
                                                                },
                                                                "Sign Out"
                                                            }
                                                        }
                                                    } else {
                                                        rsx! {
                                                            p { style: "color: rgba(255,255,255,0.6); font-size: 13px; margin: 0 0 8px 0;",
                                                                "Or use your Claude Pro/Max subscription ($200/mo credits):"
                                                            }
                                                            button {
                                                                class: "btn-primary",
                                                                style: "width: 100%; padding: 10px; font-size: 14px; font-weight: 600; border-radius: 6px; cursor: pointer;",
                                                                disabled: *oauth_loading.read(),
                                                                onclick: move |_| {
                                                                    oauth_loading.set(true);
                                                                    spawn(async move {
                                                                        let mut oauth = AnthropicOAuth::new();
                                                                        match oauth.login().await {
                                                                            Ok(()) => {
                                                                                let email = oauth.account_email().unwrap_or("").to_string();
                                                                                let tier = oauth.subscription_tier().unwrap_or("").to_string();
                                                                                oauth_status.set(("authenticated".to_string(), email, tier));
                                                                            }
                                                                            Err(e) => {
                                                                                eprintln!("[hydra:oauth] Login failed: {}", e);
                                                                                oauth_status.set(("failed".to_string(), e, String::new()));
                                                                            }
                                                                        }
                                                                        oauth_loading.set(false);
                                                                    });
                                                                },
                                                                if *oauth_loading.read() {
                                                                    "Waiting for browser..."
                                                                } else {
                                                                    "Sign in with Anthropic"
                                                                }
                                                            }
                                                            if status == "failed" {
                                                                p { style: "color: #ef4444; font-size: 12px; margin-top: 6px;",
                                                                    "Auth failed: {email}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "OpenAI" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("gpt-4o", "GPT-4o"), ("gpt-4o-mini", "GPT-4o Mini")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "model-card active" } else { "model-card" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "OpenAI API Key (sk-...)",
                                                    value: "{settings_openai_key}",
                                                    oninput: move |e| settings_openai_key.set(e.value()),
                                                }
                                                if !settings_openai_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Google" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "gemini-2.0-flash" { "model-card active" } else { "model-card" },
                                                    onclick: move |_| settings_model.set("gemini-2.0-flash".into()),
                                                    "Gemini Flash"
                                                }
                                            }
                                            div { class: "key-input-row",
                                                input {
                                                    class: "key-input",
                                                    r#type: "password",
                                                    placeholder: "Google API Key",
                                                    value: "{settings_google_key}",
                                                    oninput: move |e| settings_google_key.set(e.value()),
                                                }
                                                if !settings_google_key.read().is_empty() {
                                                    span { class: "key-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Local" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "ollama" { "model-card active" } else { "model-card" },
                                                    onclick: move |_| settings_model.set("ollama".into()),
                                                    "Ollama"
                                                }
                                            }
                                            p { class: "settings-info", "No key needed \u{2014} runs locally" }
                                        }
                                        p { class: "settings-info", "Keys saved to ~/.hydra/profile.json. Also detected from environment variables." }
                                    },
