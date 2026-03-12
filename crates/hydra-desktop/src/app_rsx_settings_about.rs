// Settings About page — Agentra Labs mission, local-first promise, complete data backup
// Included as `let settings_about = include!("app_rsx_settings_about.rs");`
|| -> Element {
    let ver = env!("CARGO_PKG_VERSION");
    let backup_text = backup_status.read().clone();
    let is_backing_up = backup_text == "running";
    rsx! {
        div { class: "about-page",
            // ── Logo + Brand ──
            div { class: "about-hero",
                div { class: "about-logo",
                    dangerous_inner_html: r#"<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20v2"/><path d="M12 2v2"/><path d="M17 20v2"/><path d="M17 2v2"/><path d="M2 12h2"/><path d="M2 17h2"/><path d="M2 7h2"/><path d="M20 12h2"/><path d="M20 17h2"/><path d="M20 7h2"/><path d="M7 20v2"/><path d="M7 2v2"/><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="8" y="8" width="8" height="8" rx="1"/></svg>"#,
                }
                div { class: "about-brand",
                    h2 { class: "about-title", "Hydra" }
                    p { class: "about-subtitle", "by Agentra Labs" }
                }
            }
            p { class: "about-version", "Version {ver} \u{00B7} Dioxus 0.6 + WebView \u{00B7} macOS" }
            // ── Mission ──
            div { class: "about-section",
                h3 { class: "about-section-title", "Our Mission" }
                p { class: "about-text",
                    "Agentra Labs builds open, local-first AI infrastructure for individuals. We believe AI should work for you \u{2014} on your machine, under your control, with memory that persists across sessions and environments."
                }
                p { class: "about-text",
                    "We focus on persistent state, structured reasoning, and policy-enforced execution so your AI agent can remember, see, and understand with continuity."
                }
            }
            // ── Local-First Promise ──
            div { class: "about-section",
                h3 { class: "about-section-title", "Your Data Stays Local" }
                div { class: "about-promise-grid",
                    div { class: "about-promise-card",
                        span { class: "about-promise-icon",
                            dangerous_inner_html: r#"<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#,
                        }
                        div { class: "about-promise-content",
                            span { class: "about-promise-title", "Private by default" }
                            span { class: "about-promise-desc", "All data stored locally in your home directory. No telemetry, no cloud sync, no tracking." }
                        }
                    }
                    div { class: "about-promise-card",
                        span { class: "about-promise-icon",
                            dangerous_inner_html: r#"<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>"#,
                        }
                        div { class: "about-promise-content",
                            span { class: "about-promise-title", "Long-lived memory" }
                            span { class: "about-promise-desc", "Your conversations, beliefs, and context persist indefinitely. No automatic expiration. Designed for 20+ years of continuity." }
                        }
                    }
                    div { class: "about-promise-card",
                        span { class: "about-promise-icon",
                            dangerous_inner_html: r#"<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>"#,
                        }
                        div { class: "about-promise-content",
                            span { class: "about-promise-title", "Portable" }
                            span { class: "about-promise-desc", "Use \"Back Up Everything\" below to export all your data \u{2014} memories, plans, decisions, config \u{2014} into a single zip. Move it to any machine." }
                        }
                    }
                }
            }
            // ── Complete Data & Backup ──
            div { class: "about-section",
                h3 { class: "about-section-title", "Your Complete Data" }
                p { class: "about-text",
                    "Hydra's 14 sisters each store your data locally. This is everything \u{2014} your memories, plans, decisions, and history. Back it all up with one click."
                }
                // Sister data directories
                { include!("app_rsx_settings_about_data.rs") }
            }
            // ── Links ──
            div { class: "about-section",
                h3 { class: "about-section-title", "Links" }
                div { class: "about-links",
                    div { class: "about-link-row",
                        span { class: "about-link-label", "Website" }
                        span { class: "about-link-value", "agentralabs.tech" }
                    }
                    div { class: "about-link-row",
                        span { class: "about-link-label", "GitHub" }
                        span { class: "about-link-value", "github.com/agentralabs" }
                    }
                    div { class: "about-link-row",
                        span { class: "about-link-label", "Contact" }
                        span { class: "about-link-value", "hello@agentralabs.tech" }
                    }
                }
            }
            p { class: "about-footer", "Open source \u{00B7} MIT License \u{00B7} Made for individuals, not corporations." }
        }
    }
}
