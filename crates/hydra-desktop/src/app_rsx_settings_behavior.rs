// Behavior settings tab — extracted from settings_b for file size compliance
// Included as `include!("app_rsx_settings_behavior.rs")`
rsx! {
    h2 { class: "settings-title", "Behavior" }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Memory" }
        p { class: "settings-desc", style: "margin-bottom: 12px;",
            "Control how Hydra's memory sister stores your conversations."
        }
        div { class: "settings-row",
            span { class: "settings-label", "Capture Mode" }
            div { class: "segmented-control",
                { let opts = [("all", "Full Conversation"), ("facts", "Facts Only"), ("none", "None")]; rsx! {
                    for (val, label) in opts.iter() {
                        button {
                            class: if *settings_memory_capture.read() == *val { "segment active" } else { "segment" },
                            onclick: { let v = val.to_string(); move |_| settings_memory_capture.set(v.clone()) },
                            "{label}"
                        }
                    }
                } }
            }
        }
        {
            let mode = settings_memory_capture.read().clone();
            let (title, desc, tradeoff) = match mode.as_str() {
                "all" => (
                    "Full Conversation",
                    "Every message, decision, and context is stored permanently. Enables \"where did we stop?\" recall across sessions.",
                    "Storage: ~2-5 KB per exchange. Best for ongoing projects where continuity matters.",
                ),
                "facts" => (
                    "Facts Only",
                    "Hydra learns your preferences, decisions, and corrections but discards raw conversation text after the session ends.",
                    "Storage: ~200-500 bytes per exchange. Balances privacy with usefulness \u{2014} Hydra still improves over time.",
                ),
                _ => (
                    "No Capture",
                    "Nothing is stored after this session. Hydra cannot learn from your interactions or recall previous context.",
                    "Storage: none. Use for sensitive or one-off conversations. Beliefs and patterns still update within the session.",
                ),
            };
            rsx! {
                div { class: "settings-row", style: "background: var(--bg-elevated); border-radius: 8px; padding: 12px; flex-direction: column; align-items: flex-start; gap: 6px;",
                    p { class: "settings-desc", style: "font-weight: 600;", "{title}" }
                    p { class: "settings-desc", style: "line-height: 1.5;", "{desc}" }
                    p { class: "settings-desc", style: "line-height: 1.5; opacity: 0.7; font-size: 12px;", "{tradeoff}" }
                }
            }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Intent Cache" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Enable intent caching" }
                span { class: "settings-desc", "Cache classified intents to skip re-classification" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_intent_cache.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_intent_cache.read(); settings_intent_cache.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_intent_cache.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Cache TTL" }
            div { class: "segmented-control",
                { let opts = ["15m", "1h", "4h", "24h"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_cache_ttl.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_cache_ttl.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Belief Revision" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Learn from corrections" }
                span { class: "settings-desc", "When you correct Hydra, it remembers for next time" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_learn_corrections.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_learn_corrections.read(); settings_learn_corrections.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_learn_corrections.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Belief persistence" }
            div { class: "segmented-control",
                { let opts = ["Session", "7 days", "30 days", "Forever"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_belief_persist.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_belief_persist.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Context & Routing" }
        div { class: "settings-row",
            span { class: "settings-label", "Compression" }
            div { class: "segmented-control",
                { let opts = ["Minimal", "Balanced", "Aggressive"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_compression.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_compression.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Dispatch" }
            div { class: "segmented-control",
                { let opts = ["Parallel", "Sequential"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_dispatch_mode.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_dispatch_mode.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Timeout" }
            div { class: "segmented-control",
                { let opts = ["5s", "10s", "30s", "60s"]; rsx! {
                    for o in opts.iter() {
                        button {
                            class: if *settings_sister_timeout.read() == *o { "segment active" } else { "segment" },
                            onclick: { let v = o.to_string(); move |_| settings_sister_timeout.set(v.clone()) },
                            "{o}"
                        }
                    }
                } }
            }
        }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Retry on failure" }
                span { class: "settings-desc", "Retry failed sister calls once" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_retry_failures.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_retry_failures.read(); settings_retry_failures.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_retry_failures.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Proactive Behavior" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Dream state" }
                span { class: "settings-desc", "Process knowledge during idle time" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_dream_state.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_dream_state.read(); settings_dream_state.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_dream_state.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Proactive insights" }
                span { class: "settings-desc", "Surface relevant info before you ask" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_proactive.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_proactive.read(); settings_proactive.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_proactive.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Email (SMTP)" }
        p { class: "settings-desc", style: "margin-bottom: 12px;",
            "Configure SMTP to let Hydra send emails. For Gmail, use an App Password."
        }
        div { class: "settings-row",
            span { class: "settings-label", "SMTP Host" }
            input { class: "settings-input", r#type: "text", placeholder: "smtp.gmail.com",
                value: "{settings_smtp_host}", oninput: move |e| settings_smtp_host.set(e.value()) }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Email / Username" }
            input { class: "settings-input", r#type: "text", placeholder: "you@gmail.com",
                value: "{settings_smtp_user}", oninput: move |e| settings_smtp_user.set(e.value()) }
        }
        div { class: "settings-row",
            span { class: "settings-label", "App Password" }
            input { class: "settings-input", r#type: "password", placeholder: "App password",
                value: "{settings_smtp_password}", oninput: move |e| settings_smtp_password.set(e.value()) }
        }
        div { class: "settings-row",
            span { class: "settings-label", "Default Recipient" }
            input { class: "settings-input", r#type: "text", placeholder: "recipient@example.com",
                value: "{settings_smtp_to}", oninput: move |e| settings_smtp_to.set(e.value()) }
        }
    }
    div { class: "settings-section",
        h3 { class: "settings-section-title", "Federation" }
        div { class: "settings-row",
            div { class: "settings-label-group",
                span { class: "settings-label", "Enable federation" }
                span { class: "settings-desc", "Connect to other Hydra instances for distributed tasks" }
            }
            div { class: "toggle-group",
                button {
                    class: if *settings_federation.read() { "toggle-track on" } else { "toggle-track" },
                    onclick: move |_| { let c = *settings_federation.read(); settings_federation.set(!c); },
                    span { class: "toggle-knob" }
                }
                { let l = if *settings_federation.read() { "On" } else { "Off" }; rsx! { span { class: "toggle-label", "{l}" } } }
            }
        }
    }
}
