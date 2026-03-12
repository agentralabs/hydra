// Pre-rendered chat controls element: approval card, celebration, error card, input bar
// Included as `let chat_controls_el: Element = include!("app_rsx_chat_controls.rs");`
rsx! {
    // Approval card
    {
        let approval = pending_approval.read();
        if let Some(card) = approval.as_ref() {
            let icon = card.icon.clone();
            let title = card.title.clone();
            let desc = card.description.clone();
            let preview_text = card.preview.clone().unwrap_or_default();
            let has_preview = card.preview.is_some();
            let show_challenge = card.needs_challenge();
            let challenge_text = card.challenge_phrase.clone().unwrap_or_default();
            let has_challenge = card.challenge_phrase.is_some();
            let primary = card.primary_action.clone();
            let secondary = card.secondary_action.clone();
            let countdown_val = *approval_countdown.read();
            let approve_mgr = card_approval_mgr.clone();
            let deny_mgr = card_approval_mgr.clone();
            let key_approve_mgr = card_approval_mgr.clone();
            let key_deny_mgr = card_approval_mgr.clone();
            rsx! {
                div {
                    class: "approval-card",
                    tabindex: "0",
                    onkeydown: move |e| {
                        match e.key() {
                            Key::Character(ref c) if c == "y" || c == "Y" => {
                                if let Some(id) = pending_approval_id.read().clone() {
                                    let _ = key_approve_mgr.submit_decision(&id, ApprovalDecision::Approved);
                                }
                                pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                            }
                            Key::Character(ref c) if c == "n" || c == "N" => {
                                if let Some(id) = pending_approval_id.read().clone() {
                                    let _ = key_deny_mgr.submit_decision(&id, ApprovalDecision::Denied { reason: "User denied via keyboard".into() });
                                }
                                pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                            }
                            _ => {}
                        }
                    },
                    div { class: "approval-header",
                        span { class: "approval-icon", "{icon}" }
                        span { class: "approval-title", "{title}" }
                    }
                    p { class: "approval-desc", "{desc}" }
                    if has_preview {
                        div { class: "approval-preview", "{preview_text}" }
                    }
                    if show_challenge && has_challenge {
                        p { class: "approval-challenge", "Type \"{challenge_text}\" to proceed" }
                        input {
                            class: if *challenge_input.read() == challenge_text { "challenge-input valid" } else { "challenge-input" },
                            value: "{challenge_input}",
                            placeholder: "{challenge_text}",
                            oninput: move |e| challenge_input.set(e.value()),
                            onkeydown: {
                                let ct = challenge_text.clone();
                                let approve_enter = card_approval_mgr.clone();
                                move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter && *challenge_input.read() == ct {
                                        if let Some(id) = pending_approval_id.read().clone() {
                                            let _ = approve_enter.submit_decision(&id, ApprovalDecision::Approved);
                                        }
                                        pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                                    }
                                }
                            },
                        }
                    }
                    if countdown_val > 0 {
                        div { class: "approval-countdown", "Auto-declining in {countdown_val}s" }
                        div {
                            class: "approval-progress-bar",
                            role: "progressbar",
                            aria_label: "Auto-decline countdown",
                            "aria-valuemin": "0",
                            "aria-valuemax": "30",
                            "aria-valuenow": "{countdown_val}",
                            div {
                                class: "approval-progress-fill",
                                style: format!("width: {}%", (countdown_val as f32 / 30.0 * 100.0).min(100.0)),
                            }
                        }
                    }
                    div { class: "approval-actions",
                        button {
                            class: "btn-primary",
                            onclick: move |_| {
                                if let Some(id) = pending_approval_id.read().clone() {
                                    let _ = approve_mgr.submit_decision(&id, ApprovalDecision::Approved);
                                }
                                pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                            },
                            "{primary} "
                            span { class: "kbd", "Y" }
                        }
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                if let Some(id) = pending_approval_id.read().clone() {
                                    let _ = deny_mgr.submit_decision(&id, ApprovalDecision::Denied { reason: "User denied".into() });
                                }
                                pending_approval.set(None); pending_approval_id.set(None); approval_countdown.set(0);
                            },
                            "{secondary} "
                            span { class: "kbd", "N" }
                        }
                    }
                }
            }
        } else {
            rsx! {}
        }
    }

    // Celebration toast (auto-dismiss after 3s)
    {
        let cel = celebration.read();
        if let Some(c) = cel.as_ref() {
            let msg = c.message.clone();
            // Auto-dismiss (only schedule once)
            if !*celebration_dismiss_scheduled.read() {
                celebration_dismiss_scheduled.set(true);
                spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    celebration.set(None);
                    celebration_dismiss_scheduled.set(false);
                });
            }
            rsx! {
                div {
                    class: "celebration-toast",
                    onclick: move |_| {
                        celebration.set(None);
                        celebration_dismiss_scheduled.set(false);
                    },
                    span { class: "celebration-check-icon" }
                    span { class: "celebration-msg", "{msg}" }
                }
            }
        } else {
            rsx! {}
        }
    }

    // Error card
    {
        let err = active_error.read();
        if let Some(error) = err.as_ref() {
            let msg = error.message.clone();
            let expl = error.explanation.clone();
            let opts: Vec<(String, bool)> = error.options.iter()
                .map(|o| (o.label.clone(), o.is_primary)).collect();
            rsx! {
                div {
                    class: "error-card",
                    div { class: "error-icon", "\u{25C9}" }
                    p { class: "error-message", "{msg}" }
                    p { class: "error-explanation", "{expl}" }
                    div { class: "error-options",
                        for (label, is_primary) in opts.iter() {
                            button {
                                class: if *is_primary { "btn-primary" } else { "btn-secondary" },
                                onclick: move |_| active_error.set(None),
                                "{label}"
                            }
                        }
                    }
                }
            }
        } else {
            rsx! {}
        }
    }

    // Input bar
    div {
        class: "input-bar",
        div { class: "input-wrapper",
            // Mic button (only when voice is enabled)
            if *settings_voice.read() {
                button {
                    class: if *voice_listening.read() { "mic-btn listening" } else { "mic-btn" },
                    title: if *voice_listening.read() { "Stop listening" } else { "Start voice input" },
                    onclick: move |_| {
                        let listening = *voice_listening.read();
                        if listening {
                            // STOP recording
                            mic_stop_flag.read().store(true, Ordering::Relaxed);
                            // voice_listening will be set false when transcript arrives
                        } else {
                            // START recording
                            let openai_key = settings_openai_key.read().clone();
                            if openai_key.is_empty() {
                                active_error.set(Some(FriendlyError {
                                    message: "Voice input requires an OpenAI API key".into(),
                                    explanation: "Go to Settings > Models and enter your OpenAI key to enable voice transcription.".into(),
                                    options: vec![],
                                    icon_state: "error".into(),
                                    can_undo: false,
                                }));
                                return;
                            }
                            voice_listening.set(true);
                            // Fresh stop flag
                            let flag = Arc::new(AtomicBool::new(false));
                            mic_stop_flag.set(flag.clone());

                            // Record in a std::thread (cpal::Stream is not Send)
                            let (tx, rx) = tokio::sync::oneshot::channel::<Option<(Vec<f32>, u32)>>();
                            std::thread::spawn(move || {
                                let result = voice_capture::record_until_stopped(flag);
                                let _ = tx.send(result);
                            });

                            // Await result, transcribe, auto-send
                            spawn(async move {
                                if let Ok(Some((samples, sample_rate))) = rx.await {
                                    if samples.len() > 1600 { // at least 0.1s of audio
                                        let wav = voice_capture::encode_wav(&samples, sample_rate);
                                        let key = settings_openai_key.read().clone();
                                        let lang = settings_stt_lang.read().clone();
                                        match voice_capture::transcribe_whisper(wav, &key, &lang).await {
                                            Ok(text) if !text.is_empty() => {
                                                input.set(text);
                                                // Auto-click send
                                                document::eval("setTimeout(function(){var b=document.querySelector('.send-btn');if(b)b.click();},50);");
                                            }
                                            Err(e) => {
                                                eprintln!("[hydra] transcription error: {}", e);
                                                active_error.set(Some(FriendlyError {
                                                    message: "Transcription failed".into(),
                                                    explanation: e,
                                                    options: vec![],
                                                    icon_state: "error".into(),
                                                    can_undo: false,
                                                }));
                                            }
                                            _ => {}
                                        }
                                    }
                                } else {
                                    active_error.set(Some(FriendlyError {
                                        message: "Microphone not available".into(),
                                        explanation: "No input device found. Check your microphone connection and system permissions.".into(),
                                        options: vec![],
                                        icon_state: "error".into(),
                                        can_undo: false,
                                    }));
                                }
                                voice_listening.set(false);
                            });
                        }
                    },
                    span { class: if *voice_listening.read() { "mic-icon listening" } else { "mic-icon" } }
                }
            }
            textarea {
                class: "chat-input",
                placeholder: if *voice_listening.read() { "Listening..." } else { "Message Hydra..." },
                value: "{input}",
                rows: "1",
                oninput: move |e| {
                    input.set(e.value());
                    if input_error.read().is_some() { input_error.set(None); }
                    document::eval("requestAnimationFrame(function(){var t=document.querySelector('.chat-input');if(t){t.style.height='auto';t.style.height=Math.min(t.scrollHeight,150)+'px';}})");
                },
                onkeydown: move |e| {
                    if e.key() == Key::Enter && !e.modifiers().shift() {
                        e.prevent_default();
                        let text = input.read().clone();
                        if !text.trim().is_empty() { send_message(text); }
                        document::eval("requestAnimationFrame(function(){var t=document.querySelector('.chat-input');if(t)t.style.height='auto';})");
                    }
                },
            }
            button {
                class: if input.read().trim().is_empty() { "send-btn disabled" } else { "send-btn" },
                disabled: input.read().trim().is_empty(),
                title: "Send message",
                aria_label: "Send message",
                onclick: move |_| {
                    // Trigger via JS — send_message closure is captured by onkeydown above
                    document::eval("document.querySelector('.chat-input')?.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))");
                },
                span {
                    class: "send-icon",
                    dangerous_inner_html: r#"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>"#,
                }
            }
        }
        {
            let err = input_error.read();
            if let Some(ref msg) = *err {
                rsx! { div { class: "input-error", "{msg}" } }
            } else {
                rsx! {}
            }
        }
        div { class: "input-footer",
            p { class: "input-hint", "Enter to send \u{00B7} Shift+Enter newline \u{00B7} \u{2318}K commands \u{00B7} \u{2318}B sidebar" }
            {
                let len = input.read().len();
                if len > 0 {
                    let count_class = if len > 9000 { "char-count warn" } else { "char-count" };
                    rsx! { span { class: count_class, "{len}/10000" } }
                } else {
                    rsx! { span { class: "powered-by", "by Agentra Labs" } }
                }
            }
        }
    }
}
