                                    "sisters" => rsx! {
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
