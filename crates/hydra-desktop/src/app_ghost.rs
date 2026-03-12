
            // ═══════════════════════════════════════════════════════
            // GHOST CURSOR OVERLAY
            // ═══════════════════════════════════════════════════════
            {
                let gc = ghost_cursor.read();
                let cursor_class = gc.css_class();
                let cursor_style = gc.transform_style();
                let label = gc.action_label.clone().unwrap_or_default();
                let has_label = gc.action_label.is_some();
                let (pdx, pdy) = gc.pupil_offset();
                let svg_html = cursor_svg(pdx, pdy);
                let is_visible = gc.visible;
                let mode_label = gc.mode.label();
                let show_mode_badge = gc.mode != CursorMode::Visible && is_visible;
                let is_replay = gc.mode == CursorMode::Replay;
                let replay_pct = (gc.replay_progress * 100.0) as u32;
                let trail_dots: Vec<(f64, f64, f64)> = gc.trail.iter().map(|d| {
                    let opacity = 1.0 - (d.age_ms as f64 / 1000.0);
                    (d.x, d.y, opacity.max(0.0))
                }).collect();
                drop(gc);

                rsx! {
                    div {
                        class: "ghost-cursor-overlay",

                        // Trail dots
                        for (i, (tx_pos, ty_pos, opacity)) in trail_dots.iter().enumerate() {
                            div {
                                key: "trail-{i}",
                                class: "ghost-trail-dot",
                                style: format!("left: {}px; top: {}px; opacity: {};", tx_pos + 4.0, ty_pos + 4.0, opacity),
                            }
                        }

                        // Click rings
                        for (i, (rx, ry, _ts)) in ghost_click_rings.read().iter().enumerate() {
                            div {
                                key: "ring-{i}",
                                class: "ghost-click-ring",
                                style: format!("left: {}px; top: {}px;", rx, ry),
                            }
                        }

                        // Robot cursor
                        div {
                            class: "{cursor_class}",
                            style: "{cursor_style}",
                            dangerous_inner_html: "{svg_html}",
                        }

                        // Action label
                        if has_label && is_visible {
                            div {
                                class: "ghost-cursor-label visible",
                                style: "{cursor_style}",
                                "{label}"
                            }
                        }

                        // Mode badge
                        if show_mode_badge {
                            div {
                                class: "ghost-mode-badge active",
                                "Cursor: {mode_label}"
                            }
                        }

                        // Replay controls
                        if is_replay {
                            div {
                                class: "ghost-replay-bar",
                                button {
                                    onclick: move |_| {
                                        ghost_cursor.write().set_mode(CursorMode::Visible);
                                    },
                                    "Stop"
                                }
                                div { class: "replay-progress",
                                    div {
                                        class: "replay-fill",
                                        style: format!("width: {}%", replay_pct),
                                    }
                                }
                                span { class: "replay-label", "{replay_pct}%" }
                            }
                        }
                    }
                }
            }
        }
    }
