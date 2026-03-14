// Settings closure for tabs: general, models, sisters
// Included as `let settings_a = include!("app_rsx_settings_a.rs");`
|tab: &str| -> Element {
    match tab {
        "general" => rsx! {
            h2 { class: "settings-title", "General" }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Appearance" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Theme" }
                        span { class: "settings-desc", "Choose your preferred color scheme" }
                    }
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
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Mode" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Default Mode" }
                        span { class: "settings-desc", "Companion: voice-first with Hydra globe. Workspace: full IDE with sidebar and panels." }
                    }
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

            // ── Frontier Model ──
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Frontier Model (Cloud)" }
                p { class: "settings-desc", "Primary model for complex tasks. Requires an API key from the provider." }
                div { class: "model-select-row",
                    select {
                        class: "model-select",
                        value: "{settings_model}",
                        onchange: move |e| settings_model.set(e.value()),
                        optgroup { label: "Anthropic",
                            option { value: "claude-opus-4-6", "Claude Opus 4.6 \u{2014} Most capable" }
                            option { value: "claude-sonnet-4-6", "Claude Sonnet 4.6 \u{2014} Fast, balanced" }
                            option { value: "claude-haiku-4-5", "Claude Haiku 4.5 \u{2014} Lightweight" }
                        }
                        optgroup { label: "OpenAI",
                            option { value: "gpt-4o", "GPT-4o \u{2014} Flagship multimodal" }
                            option { value: "gpt-4o-mini", "GPT-4o Mini \u{2014} Cost efficient" }
                            option { value: "gpt-4.1", "GPT-4.1 \u{2014} Coding specialist" }
                            option { value: "o3", "o3 \u{2014} Advanced reasoning" }
                            option { value: "o4-mini", "o4-mini \u{2014} Fast reasoning" }
                        }
                        optgroup { label: "Google",
                            option { value: "gemini-2.5-pro", "Gemini 2.5 Pro \u{2014} Thinking model" }
                            option { value: "gemini-2.5-flash", "Gemini 2.5 Flash \u{2014} Speed optimized" }
                            option { value: "gemini-2.0-flash", "Gemini 2.0 Flash \u{2014} Stable" }
                        }
                        optgroup { label: "Meta (via API)",
                            option { value: "llama-4-maverick", "Llama 4 Maverick \u{2014} 400B MoE" }
                            option { value: "llama-4-scout", "Llama 4 Scout \u{2014} 109B efficient" }
                        }
                        optgroup { label: "DeepSeek (via API)",
                            option { value: "deepseek-r1", "DeepSeek R1 \u{2014} Reasoning" }
                            option { value: "deepseek-v3", "DeepSeek V3 \u{2014} General" }
                        }
                        optgroup { label: "xAI",
                            option { value: "grok-3", "Grok 3 \u{2014} xAI flagship" }
                            option { value: "grok-3-mini", "Grok 3 Mini \u{2014} Fast reasoning" }
                        }
                        optgroup { label: "Mistral",
                            option { value: "mistral-large", "Mistral Large \u{2014} Flagship" }
                            option { value: "codestral", "Codestral \u{2014} Code specialist" }
                        }
                    }
                }
            }

            // ── Local Model ──
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Local Model (On-Device)" }
                p { class: "settings-desc", "Runs entirely on your machine. No API key, no data leaves your device." }
                div { class: "model-select-row",
                    select {
                        class: "model-select",
                        value: "{settings_local_model}",
                        onchange: move |e| settings_local_model.set(e.value()),
                        optgroup { label: "Ollama",
                            option { value: "llama3.3", "Llama 3.3 70B \u{2014} Best local general" }
                            option { value: "llama3.2", "Llama 3.2 3B \u{2014} Compact" }
                            option { value: "qwen2.5-coder", "Qwen 2.5 Coder \u{2014} Code specialist" }
                            option { value: "deepseek-r1:14b", "DeepSeek R1 14B \u{2014} Reasoning" }
                            option { value: "mistral", "Mistral 7B \u{2014} Lightweight" }
                            option { value: "gemma2", "Gemma 2 9B \u{2014} Google open" }
                            option { value: "phi4", "Phi-4 14B \u{2014} Microsoft" }
                            option { value: "codellama", "Code Llama 13B \u{2014} Code focused" }
                        }
                        optgroup { label: "LM Studio",
                            option { value: "lmstudio", "LM Studio \u{2014} Any GGUF model" }
                        }
                    }
                }
                p { class: "settings-info", "Install Ollama from ollama.com. Runs at localhost:11434." }
            }

            // ── API Keys ──
            div { class: "settings-section",
                h3 { class: "settings-section-title", "API Keys" }
                p { class: "settings-desc", "Enter keys for each provider you want to use. Keys are saved locally to ~/.hydra/profile.json and never sent anywhere." }

                // Anthropic
                div { class: "provider-key-section",
                    div { class: "provider-key-header",
                        span { class: "provider-key-name", "Anthropic" }
                        if !settings_anthropic_key.read().is_empty() {
                            span { class: "provider-key-status connected", "Connected" }
                        }
                    }
                    div { class: "key-input-row",
                        input {
                            class: "key-input",
                            r#type: "password",
                            placeholder: "sk-ant-api03-...",
                            value: "{settings_anthropic_key}",
                            oninput: move |e| settings_anthropic_key.set(e.value()),
                        }
                        if !settings_anthropic_key.read().is_empty() {
                            span { class: "key-check", "\u{2713}" }
                        }
                    }
                    // OAuth alternative
                    div { class: "oauth-section",
                        {
                            let (status, email, tier) = oauth_status.read().clone();
                            if status == "authenticated" {
                                rsx! {
                                    div { class: "oauth-status-row",
                                        span { class: "oauth-check", "\u{2713} Signed in" }
                                        if !email.is_empty() {
                                            span { class: "oauth-email", "{email}" }
                                        }
                                        if !tier.is_empty() {
                                            span { class: "oauth-tier", "{tier}" }
                                        }
                                    }
                                    button {
                                        class: "btn-secondary btn-small",
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
                                    button {
                                        class: "btn-secondary oauth-sign-in",
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
                                        if *oauth_loading.read() { "Waiting for browser..." } else { "Or sign in with Claude Pro/Max" }
                                    }
                                    if status == "failed" {
                                        p { class: "oauth-error", "Auth failed: {email}" }
                                    }
                                }
                            }
                        }
                    }
                }

                // OpenAI
                div { class: "provider-key-section",
                    div { class: "provider-key-header",
                        span { class: "provider-key-name", "OpenAI" }
                        if !settings_openai_key.read().is_empty() {
                            span { class: "provider-key-status connected", "Connected" }
                        }
                    }
                    div { class: "key-input-row",
                        input {
                            class: "key-input",
                            r#type: "password",
                            placeholder: "sk-...",
                            value: "{settings_openai_key}",
                            oninput: move |e| settings_openai_key.set(e.value()),
                        }
                        if !settings_openai_key.read().is_empty() {
                            span { class: "key-check", "\u{2713}" }
                        }
                    }
                    p { class: "settings-info", "Also used for OpenAI-compatible providers (Meta, DeepSeek) via custom endpoints." }
                }

                // Google
                div { class: "provider-key-section",
                    div { class: "provider-key-header",
                        span { class: "provider-key-name", "Google" }
                        if !settings_google_key.read().is_empty() {
                            span { class: "provider-key-status connected", "Connected" }
                        }
                    }
                    div { class: "key-input-row",
                        input {
                            class: "key-input",
                            r#type: "password",
                            placeholder: "Google AI API Key",
                            value: "{settings_google_key}",
                            oninput: move |e| settings_google_key.set(e.value()),
                        }
                        if !settings_google_key.read().is_empty() {
                            span { class: "key-check", "\u{2713}" }
                        }
                    }
                }

                // Local
                div { class: "provider-key-section",
                    div { class: "provider-key-header",
                        span { class: "provider-key-name", "Local (Ollama)" }
                        span { class: "provider-key-status", "No key needed" }
                    }
                    p { class: "settings-info", "Runs on your machine at localhost:11434. Install from ollama.com." }
                }
            }
            p { class: "settings-info", "Keys also detected from ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_API_KEY environment variables." }
        },
        _ => rsx! {
            h2 { class: "settings-title", "Sisters & MCP Servers" }
            // Built-in Sisters
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Built-in Sisters" }
                p { class: "settings-desc sisters-desc",
                    "14 specialized AI agents connected via MCP (Model Context Protocol)."
                }
                {
                    let sh = sisters.read();
                    let sister_list: Vec<(&str, &str, bool, usize)> = vec![
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
                        ("Planning", "Cognitive", sh.as_ref().map_or(false, |s| s.planning.is_some()),
                         sh.as_ref().and_then(|s| s.planning.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Cognition", "Cognitive", sh.as_ref().map_or(false, |s| s.cognition.is_some()),
                         sh.as_ref().and_then(|s| s.cognition.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
                        ("Reality", "Cognitive", sh.as_ref().map_or(false, |s| s.reality.is_some()),
                         sh.as_ref().and_then(|s| s.reality.as_ref()).map(|m| m.tools.len()).unwrap_or(0)),
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
                    let categories = ["Foundation", "Cognitive", "Astral"];
                    rsx! {
                        div { class: "sisters-total", "{connected_count}/17 connected \u{00B7} {total} tools" }
                        for cat in categories.iter() {
                            div { class: "sisters-category",
                                span { class: "sisters-cat-label", "{cat}" }
                                div { class: "sisters-grid",
                                    for (name, c, conn, tools) in sister_list.iter().filter(|(_, c, _, _)| c == cat) {
                                        div {
                                            class: if *conn { "sister-card connected" } else { "sister-card" },
                                            div { class: "sister-card-header",
                                                div { class: if *conn { "status-dot connected" } else { "status-dot" } }
                                                span { class: "sister-name", "{name}" }
                                            }
                                            { let _ = c; let st = if *conn { format!("{} tools", tools) } else { "offline".into() };
                                              rsx! { span { class: "sister-tools", "{st}" } } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "settings-info", "For external MCP servers and integrations, see the Integrations tab." }
        },
    }
}
