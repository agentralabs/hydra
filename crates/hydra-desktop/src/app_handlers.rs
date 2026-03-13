    // ── Send message handler ──
    // Wrap Arc in Signal so the closure captures only Copy types (Signal is Copy)
    let decide_sig: Signal<Arc<DecideEngine>> = use_signal(|| decide_engine.clone());
    let inv_sig: Signal<Arc<InventionEngine>> = use_signal(|| invention_engine.clone());
    let notifier_sig: Signal<Arc<parking_lot::Mutex<ProactiveNotifier>>> = use_signal(|| proactive_notifier.clone());
    let spawner_sig: Signal<Arc<AgentSpawner>> = use_signal(|| agent_spawner.clone());
    let approval_sig: Signal<Arc<ApprovalManager>> = use_signal(|| send_msg_approval_mgr.clone());
    let db_sig: Signal<Option<Arc<HydraDb>>> = use_signal(|| hydra_db.clone());
    let fed_sig: Signal<Arc<FederationManager>> = use_signal(|| federation_manager.clone());

    let mut send_message = move |text: String| {
        let validation = validate_input(&text, 10_000);
        if !validation.valid {
            input_error.set(validation.error);
            return;
        }
        input_error.set(None);
        let text = validation.trimmed;

        let is_first_message = messages.read().is_empty();
        messages.write().push(("user".into(), text.clone(), "message".into()));
        chat_db_sig.read().save_message("user", &text);
        connected.set(true);
        input.set(String::new());
        new_session_flash.set(false);

        // Rename active session to first message (truncated)
        if is_first_message {
            let cur_id = active_session_id.read().clone();
            let label = if text.len() > 40 { format!("{}...", &text[..37]) } else { text.clone() };
            if let Some(today) = sidebar.write().sections.first_mut() {
                for item in &mut today.items {
                    if item.id == cur_id {
                        item.label = label;
                        break;
                    }
                }
            }
        }
        let task_id = active_session_id.read().clone();
        is_typing.set(true);

        let anthropic_key_val = settings_anthropic_key.read().clone();
        let openai_key_val = settings_openai_key.read().clone();
        let google_key_val = settings_google_key.read().clone();
        let model_val = settings_model.read().clone();
        let user_name = onboarding.read().user_name.clone().unwrap_or_default();
        let sisters_handle = sisters.read().clone();

        let history: Vec<(String, String)> = messages.read().iter()
            .filter(|(role, _, _)| role == "user" || role == "hydra")
            .map(|(role, content, _)| {
                let api_role = if role == "user" { "user" } else { "assistant" };
                (api_role.to_string(), content.clone())
            })
            .collect();

        let loop_config = CognitiveLoopConfig {
            text,
            anthropic_key: anthropic_key_val,
            openai_key: openai_key_val,
            google_key: google_key_val,
            model: model_val,
            user_name,
            task_id: task_id.clone(),
            history,
            session_count: messages.read().len() as u32,
            anthropic_oauth_token: {
                let (status, _, _) = oauth_status.read().clone();
                if status == "authenticated" {
                    AnthropicOAuth::new().access_token().map(|s| s.to_string())
                } else {
                    None
                }
            },
            runtime: Default::default(),
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CognitiveUpdate>();

        let decide = decide_sig.read().clone();
        let inv = inv_sig.read().clone();
        let notifier = notifier_sig.read().clone();
        let spawner = spawner_sig.read().clone();
        let approval_mgr = approval_sig.read().clone();
        let db_handle = db_sig.read().clone();
        let fed_mgr = fed_sig.read().clone();
        let swarm = Some(swarm_manager.clone());
        spawn(async move { run_cognitive_loop(loop_config, sisters_handle, tx, decide, Some(undo_sig.read().clone()), Some(inv), Some(notifier), Some(spawner), Some(approval_mgr), db_handle, Some(fed_mgr), swarm).await; });

        let chat_db_rx = chat_db_sig.read().clone();
        spawn(async move {
            while let Some(update) = rx.recv().await {
                match update {
                    CognitiveUpdate::Phase(p) => phase.set(p),
                    CognitiveUpdate::IconState(s) => icon_state.set(s),
                    CognitiveUpdate::PhaseStatuses(s) => phase_statuses.set(s),
                    CognitiveUpdate::Typing(t) => is_typing.set(t),
                    CognitiveUpdate::PlanInit { goal, steps } => {
                        let step_refs: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
                        *plan_panel.write() = PlanPanel::new(&goal, step_refs);
                    }
                    CognitiveUpdate::PlanClear => { plan_panel.write().steps.clear(); }
                    CognitiveUpdate::PlanStepStart(idx) => { plan_panel.write().start_step(idx); }
                    CognitiveUpdate::PlanStepComplete { index, duration_ms } => {
                        if index == usize::MAX {
                            let n = plan_panel.read().steps.len();
                            if n > 0 { plan_panel.write().complete_step(n - 1, None, duration_ms); }
                        } else {
                            plan_panel.write().complete_step(index, None, duration_ms);
                        }
                    }
                    CognitiveUpdate::EvidenceClear => { evidence_panel.write().clear(); }
                    CognitiveUpdate::EvidenceMemory { title, content } => {
                        evidence_panel.write().add_memory_context(&title, &content);
                    }
                    CognitiveUpdate::EvidenceCode { title, content, language, file_path } => {
                        evidence_panel.write().add_code(&title, &content, language.as_deref(), file_path.as_deref(), None);
                    }
                    CognitiveUpdate::TimelineClear => { timeline_panel.write().clear(); }
                    CognitiveUpdate::Message { role, content, css_class } => {
                        chat_db_rx.save_message(&role, &content);
                        // "history-only" = already displayed via streaming, skip visible push
                        if css_class != "history-only" {
                            messages.write().push((role, content, css_class));
                        }
                    }
                    CognitiveUpdate::SidebarCompleteTask(id) => { sidebar.write().complete_task(&id); }
                    CognitiveUpdate::Celebrate(msg) => { celebration.set(Some(Celebration::small(&msg))); }
                    CognitiveUpdate::ResetIdle => {
                        phase.set("Idle".into());
                        icon_state.set("idle".into());
                        is_typing.set(false);
                        _active_progress.set(None);
                        phase_statuses.set(vec![]);
                    }
                    CognitiveUpdate::SuggestMode(mode) => { current_mode.set(mode); }
                    CognitiveUpdate::AwaitApproval { approval_id, risk_level, action, description, challenge_phrase } => {
                        // Store the approval ID so buttons can submit decisions back
                        pending_approval_id.set(approval_id);
                        // Only show approval card for medium+ risk actions.
                        // None/low risk actions proceed silently — no user interruption.
                        match risk_level.as_str() {
                            "critical" => {
                                let card = ApprovalCard::critical(&action, &description, challenge_phrase.as_deref().unwrap_or(""));
                                pending_approval.set(Some(card));
                            }
                            "high" => {
                                let card = ApprovalCard::high(&action, &description, &action);
                                pending_approval.set(Some(card));
                            }
                            "medium" => {
                                let card = ApprovalCard::medium(&action, &description);
                                pending_approval.set(Some(card));
                            }
                            _ => {
                                // none/low risk: auto-approve silently, no UI interruption
                                tracing::debug!("Auto-approved {} risk action: {}", risk_level, action);
                            }
                        }
                    }
                    CognitiveUpdate::SettingsApplied { .. } => {}
                    CognitiveUpdate::SistersCalled { .. } => {}
                    CognitiveUpdate::TokenUsage { .. } => {}
                    CognitiveUpdate::StreamChunk { .. } => {}
                    CognitiveUpdate::StreamComplete => {}
                    CognitiveUpdate::UndoStatus { can_undo: cu, can_redo: cr, last_action } => {
                        can_undo.set(cu);
                        can_redo.set(cr);
                        last_undo_action.set(last_action);
                    }
                    // ── Advanced cognitive updates (split for compilation memory) ──
                    include!("app_handlers_cognitive.rs")
                }
            }
        });
    };

    // ── Build save profile closure ──
    let save_current_profile = move || {
        let profile = PersistedProfile {
            user_name: onboarding.read().user_name.clone(),
            voice_enabled: *settings_voice.read(),
            onboarding_complete: true,
            selected_model: Some(settings_model.read().clone()),
            api_key: None,
            anthropic_api_key: { let k = settings_anthropic_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            openai_api_key: { let k = settings_openai_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            google_api_key: { let k = settings_google_key.read().clone(); if k.is_empty() { None } else { Some(k) } },
            theme: Some(settings_theme.read().clone()),
            auto_approve: *settings_auto_approve.read(),
            default_mode: Some(settings_default_mode.read().clone()),
            sounds_enabled: *settings_sounds.read(),
            sound_volume: settings_volume.read().parse::<u8>().unwrap_or(70),
            working_directory: std::env::current_dir().ok().map(|p| p.display().to_string()),
        };
        save_profile(&profile);
    };

    // ══════════════════════════════════════════════
    //  RSX — every class name matches styles.css
    // ══════════════════════════════════════════════
