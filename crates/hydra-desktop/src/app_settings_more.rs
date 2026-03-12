                                    "voice" => rsx! {
                                        h2 { class: "settings-title", "Voice & Audio" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Voice" }
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
                                                    {
                                                        let label = if *settings_voice.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
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
                                                    {
                                                        let label = if *settings_sounds.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Volume" }
                                                input {
                                                    class: "settings-slider",
                                                    r#type: "range",
                                                    min: "0", max: "100",
                                                    value: "{settings_volume}",
                                                    oninput: move |e| settings_volume.set(e.value()),
                                                }
                                            }
                                        }
                                        p { class: "settings-info", "STT: Whisper (local). TTS: Piper (local). Wake word detection supported." }
                                    },
                                    "policies" => rsx! {
                                        h2 { class: "settings-title", "Safety & Policies" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Execution Gate" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Auto-approve low-risk actions" }
                                                    span { class: "settings-desc", "Skip approval for actions classified as low risk" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: if *settings_auto_approve.read() { "toggle-track on" } else { "toggle-track" },
                                                        onclick: move |_| { let c = *settings_auto_approve.read(); settings_auto_approve.set(!c); },
                                                        span { class: "toggle-knob" }
                                                    }
                                                    {
                                                        let label = if *settings_auto_approve.read() { "On" } else { "Off" };
                                                        rsx! { span { class: "toggle-label", "{label}" } }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Safety Stack" }
                                            p { class: "settings-info",
                                                "Execution Gate evaluates risk before every action. Kill Switch provides emergency stop (Cmd+Shift+K). Boundary Enforcer sets hard limits on file system, network, and process access."
                                            }
                                        }
                                    },
                                    "behavior" => rsx! {
                                        h2 { class: "settings-title", "Behavior" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Intent Cache" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Enable intent caching" }
                                                    span { class: "settings-desc", "Cache classified intents to skip re-classification" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Cache TTL" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "15m" }
                                                    button { class: "segment active", "1h" }
                                                    button { class: "segment", "4h" }
                                                    button { class: "segment", "24h" }
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
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Belief persistence" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "Session" }
                                                    button { class: "segment active", "7 days" }
                                                    button { class: "segment", "30 days" }
                                                    button { class: "segment", "Forever" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Context Compression" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Compression strategy" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "Minimal" }
                                                    button { class: "segment active", "Balanced" }
                                                    button { class: "segment", "Aggressive" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Sister Routing" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Dispatch mode" }
                                                div { class: "segmented-control",
                                                    button { class: "segment active", "Parallel" }
                                                    button { class: "segment", "Sequential" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Sister timeout" }
                                                div { class: "segmented-control",
                                                    button { class: "segment", "5s" }
                                                    button { class: "segment active", "10s" }
                                                    button { class: "segment", "30s" }
                                                    button { class: "segment", "60s" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Retry on failure" }
                                                    span { class: "settings-desc", "Retry failed sister calls once before giving up" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Proactive Behavior" }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Dream state" }
                                                    span { class: "settings-desc", "Process and consolidate knowledge during idle time" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                            div { class: "settings-row",
                                                div { class: "settings-label-group",
                                                    span { class: "settings-label", "Proactive insights" }
                                                    span { class: "settings-desc", "Surface relevant information before you ask" }
                                                }
                                                div { class: "toggle-group",
                                                    button {
                                                        class: "toggle-track on",
                                                        span { class: "toggle-knob" }
                                                    }
                                                    span { class: "toggle-label", "On" }
                                                }
                                            }
                                        }
                                    },
                                    _ => rsx! {
                                        h2 { class: "settings-title", "Advanced" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Server" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "HTTP Server" }
                                                span { class: "settings-desc", style: "color: var(--success);", "http://127.0.0.1:3100" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "SSE Events" }
                                                span { class: "settings-desc", "/events" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "JSON-RPC" }
                                                span { class: "settings-desc", "/rpc" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "File Paths" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Config" }
                                                span { class: "settings-desc", "~/.hydra/config.toml" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Profile" }
                                                span { class: "settings-desc", "~/.hydra/profile.json" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Database" }
                                                span { class: "settings-desc", "~/.hydra/hydra.db" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Sessions" }
                                                span { class: "settings-desc", "~/.hydra/sessions/" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "MCP Config" }
                                                span { class: "settings-desc", "~/.hydra/mcp.json" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Receipts" }
                                                span { class: "settings-desc", "~/.hydra/receipts/" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Engine" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Cognitive Loop" }
                                                span { class: "settings-desc", "5-phase: Perceive \u{2192} Think \u{2192} Decide \u{2192} Act \u{2192} Learn" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Execution Gate" }
                                                span { class: "settings-desc", "6-layer security stack" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Receipt Ledger" }
                                                span { class: "settings-desc", "Hash-chained audit trail with tamper detection" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Kill Switch" }
                                                span { class: "settings-desc", "Cmd+Shift+K \u{2014} emergency halt" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Skills & Federation" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Crystallized Skills" }
                                                span { class: "settings-desc", "Evolve sister auto-captures reusable patterns" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Federation" }
                                                span { class: "settings-desc", "Peer discovery, skill sharing, task delegation" }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Undo Stack" }
                                                span { class: "settings-desc", "Cmd+Z to undo file actions \u{2014} bounded history" }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Debug" }
                                            p { class: "settings-info", "Version: Hydra v1.0.0" }
                                            p { class: "settings-info", "Runtime: Dioxus 0.6 + WebView" }
                                            p { class: "settings-info", "Platform: macOS (Darwin)" }
                                        }
                                    },
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
