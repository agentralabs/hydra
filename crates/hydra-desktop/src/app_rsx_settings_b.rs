// Settings closure for tabs: voice, policies, behavior, advanced
// Included as `let settings_b = include!("app_rsx_settings_b.rs");`
|tab: &str| -> Element {
    match tab {
        "voice" => rsx! {
            h2 { class: "settings-title", "Voice & Audio" }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Speech" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Voice Mode" }
                        span { class: "settings-desc", "Enable speech-to-text and text-to-speech" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_voice.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_voice.read(); settings_voice.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let label = if *settings_voice.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{label}" } } }
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "TTS Voice" }
                    select {
                        class: "model-select",
                        value: "{settings_tts_voice}",
                        onchange: move |e| settings_tts_voice.set(e.value()),
                        optgroup { label: "Piper (Local)",
                            option { value: "nova", "Nova \u{2014} Warm, conversational" }
                            option { value: "amy", "Amy \u{2014} Clear, professional" }
                            option { value: "ryan", "Ryan \u{2014} Neutral, calm" }
                            option { value: "jenny", "Jenny \u{2014} Friendly, bright" }
                            option { value: "lessac", "Lessac \u{2014} Precise, articulate" }
                        }
                        optgroup { label: "System",
                            option { value: "system-default", "System Default" }
                        }
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "STT Language" }
                    select {
                        class: "model-select",
                        value: "{settings_stt_lang}",
                        onchange: move |e| settings_stt_lang.set(e.value()),
                        option { value: "en", "English" }
                        option { value: "es", "Spanish" }
                        option { value: "fr", "French" }
                        option { value: "de", "German" }
                        option { value: "ja", "Japanese" }
                        option { value: "zh", "Chinese (Mandarin)" }
                        option { value: "ko", "Korean" }
                        option { value: "pt", "Portuguese" }
                        option { value: "auto", "Auto-detect" }
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Audio Input" }
                    select {
                        class: "model-select",
                        value: "{settings_audio_input}",
                        onchange: move |e| settings_audio_input.set(e.value()),
                        option { value: "default", "System Default" }
                        option { value: "builtin", "Built-in Microphone" }
                        option { value: "external", "External Microphone" }
                    }
                }
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Listening" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Wake Word" }
                        span { class: "settings-desc", "Say \"Hey Hydra\" to activate voice input" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_wake_word.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_wake_word.read(); settings_wake_word.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_wake_word.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Auto-listen after response" }
                        span { class: "settings-desc", "Keep microphone active after Hydra speaks" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_auto_listen.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_auto_listen.read(); settings_auto_listen.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_auto_listen.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Effects" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Sound Effects" }
                        span { class: "settings-desc", "Play sounds for notifications and events" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_sounds.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_sounds.read(); settings_sounds.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_sounds.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Volume" }
                    input {
                        class: "settings-slider", r#type: "range",
                        min: "0", max: "100", value: "{settings_volume}",
                        oninput: move |e| settings_volume.set(e.value()),
                    }
                }
            }
            p { class: "settings-info", "All voice processing runs locally. STT: Whisper. TTS: Piper. No audio leaves your machine." }
        },
        "policies" => rsx! {
            h2 { class: "settings-title", "Safety & Policies" }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Risk Threshold" }
                p { class: "settings-desc", style: "margin-bottom: 12px;",
                    "Control how much autonomy Hydra has. Higher thresholds mean fewer approval prompts."
                }
                div { class: "settings-row",
                    span { class: "settings-label", "Auto-approve up to" }
                    div { class: "segmented-control",
                        { let opts = ["none", "low", "medium", "high"]; rsx! {
                            for o in opts.iter() {
                                button {
                                    class: if *settings_risk_threshold.read() == *o { "segment active" } else { "segment" },
                                    onclick: { let v = o.to_string(); move |_| settings_risk_threshold.set(v.clone()) },
                                    { match *o { "none" => "None", "low" => "Low", "medium" => "Medium", _ => "High" } }
                                }
                            }
                        } }
                    }
                }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Always approve critical actions" }
                        span { class: "settings-desc", "Destructive operations always require confirmation" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_require_approval_critical.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_require_approval_critical.read(); settings_require_approval_critical.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_require_approval_critical.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Permissions" }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "File writes" }
                        span { class: "settings-desc", "Allow Hydra to create, edit, and delete files" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_file_write.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_file_write.read(); settings_file_write.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_file_write.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Network access" }
                        span { class: "settings-desc", "Allow outbound HTTP requests and API calls" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_network_access.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_network_access.read(); settings_network_access.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_network_access.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Shell execution" }
                        span { class: "settings-desc", "Allow running terminal commands" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_shell_exec.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_shell_exec.read(); settings_shell_exec.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_shell_exec.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Limits" }
                div { class: "settings-row",
                    span { class: "settings-label", "Max file edits per turn" }
                    div { class: "segmented-control",
                        { let opts = ["5", "10", "25", "50", "unlimited"]; rsx! {
                            for o in opts.iter() {
                                button {
                                    class: if *settings_max_file_edits.read() == *o { "segment active" } else { "segment" },
                                    onclick: { let v = o.to_string(); move |_| settings_max_file_edits.set(v.clone()) },
                                    "{o}"
                                }
                            }
                        } }
                    }
                }
                div { class: "settings-row",
                    div { class: "settings-label-group",
                        span { class: "settings-label", "Sandbox mode" }
                        span { class: "settings-desc", "Run actions in isolated environment (experimental)" }
                    }
                    div { class: "toggle-group",
                        button {
                            class: if *settings_sandbox_mode.read() { "toggle-track on" } else { "toggle-track" },
                            onclick: move |_| { let c = *settings_sandbox_mode.read(); settings_sandbox_mode.set(!c); },
                            span { class: "toggle-knob" }
                        }
                        { let l = if *settings_sandbox_mode.read() { "On" } else { "Off" };
                          rsx! { span { class: "toggle-label", "{l}" } } }
                    }
                }
            }
            div { class: "settings-section",
                h3 { class: "settings-section-title", "Emergency Controls" }
                div { class: "policy-emergency-row",
                    div { class: "policy-emergency-icon", "\u{26A0}" }
                    div { class: "policy-emergency-info",
                        span { class: "settings-label", "Kill Switch" }
                        span { class: "settings-desc", "\u{2318}\u{21E7}K \u{2014} Immediately halts all actions, kills child processes, cancels pending approvals." }
                    }
                }
            }
        },
        "behavior" => {
            let behavior_content = include!("app_rsx_settings_behavior.rs");
            behavior_content
        },
        _ => {
            let advanced_content = include!("app_rsx_settings_advanced.rs");
            advanced_content
        },
    }
}
