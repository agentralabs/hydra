// Companion mode — pure voice platform with animated globe.
// Typing seamlessly transitions to chat mode (the interaction IS the switch).
// Included as `let companion_el: Element = include!("app_rsx_companion.rs");`
rsx! {
    div {
        class: "companion-mode",

        // ── Ambient background gradient ──
        div { class: "companion-bg" }

        // ── Greeting / prompt ──
        {
            let p = phase.read();
            let name = onboarding.read().user_name.clone().unwrap_or_default();
            let personal_greeting = match p.as_str() {
                "Perceive" | "Think" | "Decide" | "Act" | "Learn" | "Done" | "Error" => String::new(),
                _ => {
                    if name.is_empty() {
                        "What can I help with?".to_string()
                    } else {
                        format!("What can I help with, {}?", name)
                    }
                }
            };
            if !personal_greeting.is_empty() {
                rsx! { p { class: "companion-greeting", "{personal_greeting}" } }
            } else {
                rsx! {}
            }
        }

        // ── Globe with animation wrapper ──
        {
            let p = phase.read();
            let has_approval = pending_approval.read().is_some();
            let voice_on = *settings_voice.read();
            let listening = *voice_listening.read();
            let globe_state = derive_globe_state(&p, has_approval, voice_on, listening);
            let params = globe_params(globe_state);
            let svg_html = globe_svg(&params, GlobeSize::Full.pixels());
            let anim_class = format!("companion-globe {}", params.animation);
            rsx! {
                div {
                    class: anim_class,
                    dangerous_inner_html: svg_html,
                }
            }
        }

        // ── Status text ──
        {
            let p = phase.read();
            let listening = *voice_listening.read();
            let status = match p.as_str() {
                "Perceive" | "Think" => "Thinking...",
                "Decide" => "Deciding...",
                "Act" => "Working...",
                "Learn" => "Learning...",
                "Done" => "Done",
                "Error" => "Something went wrong",
                _ => if listening { "Listening..." } else { "Ready" },
            };
            let status_class = match p.as_str() {
                "Error" => "companion-status error",
                "Done" => "companion-status done",
                _ => if listening { "companion-status listening" } else { "companion-status" },
            };
            rsx! { p { class: status_class, "{status}" } }
        }

        // ── Last response transcript ──
        {
            let msgs = messages.read();
            let last_hydra = msgs.iter().rev()
                .find(|(role, _, _)| role == "assistant")
                .map(|(_, content, _)| {
                    if content.len() > 200 {
                        format!("{}...", &content[..197])
                    } else {
                        content.clone()
                    }
                });
            if let Some(ref text) = last_hydra {
                rsx! {
                    div {
                        class: "companion-transcript",
                        p { class: "companion-transcript-text", "{text}" }
                    }
                }
            } else {
                rsx! {}
            }
        }

        // ── Mic button — direct voice capture (not delegated to hidden button) ──
        div {
            class: "companion-controls",
            button {
                class: if *voice_listening.read() { "companion-mic active" } else { "companion-mic" },
                title: if *voice_listening.read() { "Stop listening" } else { "Talk to Hydra" },
                onclick: move |_| { toggle_voice.call(()); },
                if *voice_listening.read() {
                    span {
                        class: "companion-mic-icon",
                        dangerous_inner_html: r#"<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>"#,
                    }
                } else {
                    span {
                        class: "companion-mic-icon",
                        dangerous_inner_html: r#"<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="2" width="6" height="12" rx="3"/><path d="M5 10a7 7 0 0 0 14 0"/><line x1="12" y1="19" x2="12" y2="22"/><line x1="8" y1="22" x2="16" y2="22"/></svg>"#,
                    }
                }
            }
            // Visual feedback when listening
            if *voice_listening.read() {
                div { class: "companion-listening-indicator",
                    span { class: "companion-pulse-dot" }
                    span { class: "companion-pulse-dot" }
                    span { class: "companion-pulse-dot" }
                }
            }
        }

        // ── Seamless text bridge — typing here transitions to chat ──
        div {
            class: "companion-bridge",
            span { class: "companion-bridge-divider" }
            span { class: "companion-bridge-label", "or type" }
            span { class: "companion-bridge-divider" }
        }
        div {
            class: "companion-input-area",
            input {
                class: "companion-input",
                placeholder: "Type a message...",
                value: "{input}",
                oninput: move |e| { input.set(e.value()); },
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        let text = input.read().clone();
                        if !text.is_empty() {
                            current_mode.set("workspace".into());
                            show_sidebar.set(true);
                            // Dispatch to main chat input — send_message is captured there
                            document::eval("setTimeout(function(){var t=document.querySelector('.chat-input');if(t){t.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))}},50)");
                        }
                    }
                },
            }
        }

        p { class: "companion-hint", "Tap mic to start \u{00B7} Hydra listens and responds hands-free \u{00B7} \u{2318}K commands" }
    }
}
