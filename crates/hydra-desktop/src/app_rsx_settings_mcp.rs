// Settings MCP page — integrations, popular servers, marketplace
// Included as `let settings_mcp = include!("app_rsx_settings_mcp.rs");`
|| -> Element {
    rsx! {
        h2 { class: "settings-title", "Integrations" }

        // ── Popular MCP Servers ──
        div { class: "settings-section",
            h3 { class: "settings-section-title", "Popular Integrations" }
            p { class: "settings-desc",
                "Connect Hydra to your favorite tools via MCP (Model Context Protocol). Each integration gives Hydra new capabilities."
            }
            div { class: "mcp-catalog",
                // Row 1: Dev tools
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "GitHub" }
                            span { class: "mcp-item-desc", "Issues, PRs, repos, code search" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-github" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><polyline points="13 2 13 9 20 9"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Filesystem" }
                            span { class: "mcp-item-desc", "Read, write, search local files" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-filesystem" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Puppeteer" }
                            span { class: "mcp-item-desc", "Browser automation, screenshots" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-puppeteer" }
                }
                // Row 2: Design & Comms
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="2"/><path d="M12 2v4"/><path d="M12 18v4"/><path d="M4.93 4.93l2.83 2.83"/><path d="M16.24 16.24l2.83 2.83"/><path d="M2 12h4"/><path d="M18 12h4"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Figma" }
                            span { class: "mcp-item-desc", "Design files, components, styles" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx figma-developer-mcp" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Slack" }
                            span { class: "mcp-item-desc", "Channels, messages, search" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-slack" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "PostgreSQL" }
                            span { class: "mcp-item-desc", "Query databases, inspect schemas" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-postgres" }
                }
                // Row 3: Data & Productivity
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Brave Search" }
                            span { class: "mcp-item-desc", "Web and local search" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx @modelcontextprotocol/server-brave-search" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Notion" }
                            span { class: "mcp-item-desc", "Pages, databases, search workspace" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx notion-mcp-server" }
                }
                div { class: "mcp-item",
                    div { class: "mcp-item-header",
                        span { class: "mcp-item-icon",
                            dangerous_inner_html: r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/></svg>"#,
                        }
                        div { class: "mcp-item-info",
                            span { class: "mcp-item-name", "Linear" }
                            span { class: "mcp-item-desc", "Issues, projects, team workflows" }
                        }
                    }
                    span { class: "mcp-item-badge", "npx linear-mcp-server" }
                }
            }
        }

        // ── MCP Marketplace link ──
        div { class: "settings-section",
            h3 { class: "settings-section-title", "Discover More" }
            p { class: "settings-desc",
                "Browse hundreds of community MCP servers for any tool or service."
            }
            div { class: "mcp-marketplace-card",
                div { class: "mcp-marketplace-content",
                    span { class: "mcp-marketplace-title", "MCP Server Directory" }
                    span { class: "mcp-marketplace-desc", "github.com/modelcontextprotocol/servers" }
                }
                span { class: "mcp-marketplace-arrow", "\u{2192}" }
            }
            div { class: "mcp-marketplace-card",
                div { class: "mcp-marketplace-content",
                    span { class: "mcp-marketplace-title", "Smithery MCP Registry" }
                    span { class: "mcp-marketplace-desc", "smithery.ai \u{2014} install, test, and deploy MCP servers" }
                }
                span { class: "mcp-marketplace-arrow", "\u{2192}" }
            }
        }

        // ── Your Configured Servers ──
        div { class: "settings-section",
            h3 { class: "settings-section-title", "Your MCP Servers" }
            p { class: "settings-desc",
                "Servers configured in ~/.hydra/mcp.json are auto-connected at startup."
            }
            div { class: "mcp-config-path",
                span { class: "settings-label", "Config" }
                span { class: "settings-desc", "~/.hydra/mcp.json" }
            }
            div { class: "mcp-actions",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        #[cfg(target_os = "macos")]
                        { let _ = std::process::Command::new("open").arg("-e")
                            .arg(format!("{}/.hydra/mcp.json", std::env::var("HOME").unwrap_or_default()))
                            .spawn(); }
                    },
                    "Edit mcp.json"
                }
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        let home = std::env::var("HOME").unwrap_or_default();
                        let path = format!("{}/.hydra/mcp.json", home);
                        if !std::path::Path::new(&path).exists() {
                            let dir = format!("{}/.hydra", home);
                            let _ = std::fs::create_dir_all(&dir);
                            let _ = std::fs::write(&path, "{\n  \"mcpServers\": {}\n}\n");
                        }
                        #[cfg(target_os = "macos")]
                        { let _ = std::process::Command::new("open").arg("-R").arg(&path).spawn(); }
                    },
                    "Reveal in Finder"
                }
            }
            p { class: "settings-info",
                "Add via CLI: hydra mcp add my-server -- npx my-mcp-server"
            }
        }

        // ── Skills & Adapters ──
        div { class: "settings-section",
            h3 { class: "settings-section-title", "Adapters" }
            div { class: "settings-row",
                div { class: "settings-label-group",
                    span { class: "settings-label", "MCP Adapter" }
                    span { class: "settings-desc", "Auto-converts MCP tools to Hydra skills" }
                }
                span { class: "settings-badge active", "Active" }
            }
            div { class: "settings-row",
                div { class: "settings-label-group",
                    span { class: "settings-label", "Crystallized Skills" }
                    span { class: "settings-desc", "Auto-captured patterns from Evolve sister" }
                }
                span { class: "settings-badge active", "Active" }
            }
            div { class: "settings-row",
                div { class: "settings-label-group",
                    span { class: "settings-label", "OpenClaw Adapter" }
                    span { class: "settings-desc", "Import skills from OpenClaw registry" }
                }
                span { class: "settings-badge", "Planned" }
            }
        }

        p { class: "settings-info", "MCP supports stdio, HTTP, and WebSocket transports. All servers run locally." }
    }
}
