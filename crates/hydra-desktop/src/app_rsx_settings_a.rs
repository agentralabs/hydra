// Settings closure for tabs: general, models, sisters
// Included as `let settings_a = include!("app_rsx_settings_a.rs");`
|tab: &str| -> Element {
    match tab {
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
        _ => rsx! {
            // sisters tab
            h2 { class: "settings-title", "Sisters & MCP" }
            div { class: "settings-section",
                p { class: "settings-info", style: "margin-bottom: 16px;",
                    "Hydra connects to 14 sister agents via MCP (Model Context Protocol). Each sister is a specialized AI tool server."
                }
                {
                    let sh = sisters.read();
                    // Build full 14-sister list: (name, category, connected, tool_count)
                    let sister_list: Vec<(&str, &str, bool, usize)> = vec![
                        // Foundation Sisters (7)
                        ("Memory", "Foundation", sh.as_ref().map_or(false, |s| s.memory.is_some()),
                         sh.as_ref().and_then(|s| s.memory.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Identity", "Foundation", sh.as_ref().map_or(false, |s| s.identity.is_some()),
                         sh.as_ref().and_then(|s| s.identity.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Codebase", "Foundation", sh.as_ref().map_or(false, |s| s.codebase.is_some()),
                         sh.as_ref().and_then(|s| s.codebase.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Vision", "Foundation", sh.as_ref().map_or(false, |s| s.vision.is_some()),
                         sh.as_ref().and_then(|s| s.vision.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Comm", "Foundation", sh.as_ref().map_or(false, |s| s.comm.is_some()),
                         sh.as_ref().and_then(|s| s.comm.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Contract", "Foundation", sh.as_ref().map_or(false, |s| s.contract.is_some()),
                         sh.as_ref().and_then(|s| s.contract.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Time", "Foundation", sh.as_ref().map_or(false, |s| s.time.is_some()),
                         sh.as_ref().and_then(|s| s.time.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        // Cognitive Sisters (3)
                        ("Planning", "Cognitive", sh.as_ref().map_or(false, |s| s.planning.is_some()),
                         sh.as_ref().and_then(|s| s.planning.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Cognition", "Cognitive", sh.as_ref().map_or(false, |s| s.cognition.is_some()),
                         sh.as_ref().and_then(|s| s.cognition.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Reality", "Cognitive", sh.as_ref().map_or(false, |s| s.reality.is_some()),
                         sh.as_ref().and_then(|s| s.reality.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        // Astral Sisters (4)
                        ("Forge", "Astral", sh.as_ref().map_or(false, |s| s.forge.is_some()),
                         sh.as_ref().and_then(|s| s.forge.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Aegis", "Astral", sh.as_ref().map_or(false, |s| s.aegis.is_some()),
                         sh.as_ref().and_then(|s| s.aegis.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Veritas", "Astral", sh.as_ref().map_or(false, |s| s.veritas.is_some()),
                         sh.as_ref().and_then(|s| s.veritas.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Evolve", "Astral", sh.as_ref().map_or(false, |s| s.evolve.is_some()),
                         sh.as_ref().and_then(|s| s.evolve.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                    ];
                    let total: usize = sister_list.iter().map(|(_, _, _, t)| *t).sum();
                    let connected_count = sister_list.iter().filter(|(_, _, c, _)| *c).count();
                    rsx! {
                        div { class: "sisters-total", "{connected_count}/14 sisters connected \u{00B7} {total} total tools" }
                        // Foundation
                        h3 { class: "settings-section-title", style: "margin-top: 16px;", "Foundation Sisters" }
                        div { class: "sisters-grid",
                            for (name, cat, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Foundation") {
                                div {
                                    class: "sister-card",
                                    div { class: "sister-card-header",
                                        div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                        span { class: "sister-name", "{name}" }
                                    }
                                    {
                                        let _ = cat;
                                        let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                        rsx! { span { class: "sister-tools", "{status_text}" } }
                                    }
                                }
                            }
                        }
                        // Cognitive
                        h3 { class: "settings-section-title", style: "margin-top: 16px;", "Cognitive Sisters" }
                        div { class: "sisters-grid",
                            for (name, _, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Cognitive") {
                                div {
                                    class: "sister-card",
                                    div { class: "sister-card-header",
                                        div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                        span { class: "sister-name", "{name}" }
                                    }
                                    {
                                        let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                        rsx! { span { class: "sister-tools", "{status_text}" } }
                                    }
                                }
                            }
                        }
                        // Astral
                        h3 { class: "settings-section-title", style: "margin-top: 16px;", "Astral Sisters" }
                        div { class: "sisters-grid",
                            for (name, _, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| *c == "Astral") {
                                div {
                                    class: "sister-card",
                                    div { class: "sister-card-header",
                                        div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                        span { class: "sister-name", "{name}" }
                                    }
                                    {
                                        let status_text = if *conn { format!("{} tools", tools) } else { "offline".to_string() };
                                        rsx! { span { class: "sister-tools", "{status_text}" } }
                                    }
                                }
                            }
                        }
                        p { class: "settings-info", style: "margin-top: 16px;",
                            "External MCP servers can be added via ~/.hydra/mcp.json"
                        }
                    }
                }
            }
        },
    }
}
