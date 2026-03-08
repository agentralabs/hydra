//! Hydra Desktop — Dioxus App component and launcher.
//!
//! Extracted from main.rs to keep the binary entry point slim.

#![cfg(feature = "desktop")]

use dioxus::prelude::*;
use crate::components::onboarding::{OnboardingState, OnboardingStep};
use crate::components::approval::ApprovalCard;
use crate::components::progress::{Celebration, ProgressJourney};
use crate::components::error_display::FriendlyError;
use crate::components::sidebar::Sidebar;
use crate::components::input::validate_input;
use crate::components::phases::{build_phase_dots, build_connectors};
use crate::components::command_palette::CommandPalette;
use crate::state::hydra::PhaseStatus;
use crate::components::plan_panel::{PlanPanel, StepStatus};
use crate::components::timeline_panel::{TimelinePanel, TimelineEventKind};
use crate::components::evidence_panel::EvidencePanel;
use crate::profile::{load_profile, save_profile, PersistedProfile};
use crate::sisters::SistersHandle;
use crate::cognitive::{CognitiveLoopConfig, CognitiveUpdate, run_cognitive_loop};
use crate::utils::markdown::markdown_to_html;

const CSS: &str = include_str!("styles.css");

#[allow(non_snake_case)]
fn App() -> Element {
    // Load persisted profile on first render
    let persisted = load_profile();
    let onboarding_done = persisted.as_ref().map_or(false, |p| p.onboarding_complete);

    // Initialize sisters on first render — spawn MCP processes
    let sisters: Signal<Option<SistersHandle>> = use_signal(|| None);
    let sisters_status = use_signal(|| "Connecting...".to_string());
    {
        let mut sisters = sisters.clone();
        let mut sisters_status = sisters_status.clone();
        use_hook(move || {
            spawn(async move {
                let handle = crate::sisters::init_sisters().await;
                let status = handle.status_summary();
                sisters.set(Some(handle));
                sisters_status.set(status);
            });
        });
    }

    // Extract settings from persisted profile upfront (before persisted is moved)
    let init_theme = persisted.as_ref().and_then(|p| p.theme.clone()).unwrap_or_else(|| "dark".to_string());
    let init_voice = persisted.as_ref().map_or(false, |p| p.voice_enabled);
    let init_sounds = persisted.as_ref().map_or(true, |p| p.sounds_enabled);
    let init_volume = persisted.as_ref().map_or("70".to_string(), |p| p.sound_volume.to_string());
    let init_auto_approve = persisted.as_ref().map_or(false, |p| p.auto_approve);
    let init_default_mode = persisted.as_ref().and_then(|p| p.default_mode.clone()).unwrap_or_else(|| "companion".to_string());
    let init_model = persisted.as_ref().and_then(|p| p.selected_model.clone()).unwrap_or_else(|| "claude-sonnet-4-6".to_string());
    // Load API keys: profile first, then env var fallback
    let init_anthropic_key = persisted.as_ref()
        .and_then(|p| p.anthropic_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let init_openai_key = persisted.as_ref()
        .and_then(|p| p.openai_api_key.clone())
        .or_else(|| persisted.as_ref().and_then(|p| p.api_key.clone()).filter(|k| k.starts_with("sk-") && !k.starts_with("sk-ant-")))
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let init_google_key = persisted.as_ref()
        .and_then(|p| p.google_api_key.clone())
        .or_else(|| std::env::var("GOOGLE_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();
    let init_api_key = String::new(); // Legacy field, kept for compat

    // Core state
    let mut input = use_signal(|| String::new());
    let mut messages = use_signal(|| Vec::<(String, String, String)>::new()); // (role, content, css_class)
    let mut connected = use_signal(|| false);
    let mut phase = use_signal(|| "Idle".to_string());
    let mut icon_state = use_signal(|| "idle".to_string());

    // Onboarding state — restore from profile if available
    let mut onboarding = use_signal(move || {
        if let Some(ref p) = persisted {
            let mut state = OnboardingState::new();
            if let Some(ref name) = p.user_name {
                state.set_name(name);
            }
            if p.voice_enabled {
                state.enable_voice();
            }
            if p.onboarding_complete {
                // Fast-forward to Complete
                state.advance(); // Intro -> AskName
                state.advance(); // AskName -> AskVoice
                state.advance(); // AskVoice -> Complete
            }
            state
        } else {
            OnboardingState::new()
        }
    });
    let mut show_onboarding = use_signal(move || !onboarding_done);

    // Settings state
    let mut show_settings = use_signal(|| false);
    let mut settings_theme = use_signal(move || init_theme);
    let mut settings_voice = use_signal(move || init_voice);
    let mut settings_sounds = use_signal(move || init_sounds);
    let mut settings_volume = use_signal(move || init_volume);
    let mut settings_auto_approve = use_signal(move || init_auto_approve);
    let init_mode_for_current = init_default_mode.clone();
    let init_mode_for_sidebar = init_default_mode.clone();
    let mut settings_default_mode = use_signal(move || init_default_mode);
    let mut settings_model = use_signal(move || init_model);
    let _settings_api_key = use_signal(move || init_api_key); // Legacy, kept for profile compat
    let mut settings_anthropic_key = use_signal(move || init_anthropic_key);
    let mut settings_openai_key = use_signal(move || init_openai_key);
    let mut settings_google_key = use_signal(move || init_google_key);

    // Mode state — initialized from saved profile
    let mut current_mode = use_signal(move || init_mode_for_current);

    // Sidebar state
    let mut sidebar = use_signal(Sidebar::new);
    let mut show_sidebar = use_signal(move || init_mode_for_sidebar == "workspace");

    // Approval state
    let mut pending_approval = use_signal(|| Option::<ApprovalCard>::None);
    let mut challenge_input = use_signal(|| String::new());

    // Progress state
    let mut active_progress = use_signal(|| Option::<ProgressJourney>::None);
    let mut celebration = use_signal(|| Option::<Celebration>::None);

    // Error state
    let mut active_error = use_signal(|| Option::<FriendlyError>::None);

    // Features drawer state
    let mut show_features = use_signal(|| false);

    // Typing indicator state
    let mut is_typing = use_signal(|| false);

    // Phase tracking state for phase dots
    let mut phase_statuses = use_signal(|| Vec::<PhaseStatus>::new());

    // Input validation error
    let mut input_error = use_signal(|| Option::<String>::None);

    // Approval countdown state
    let mut approval_countdown = use_signal(|| 0u32);

    // Workspace panel state
    let mut plan_panel = use_signal(|| PlanPanel::default());
    let mut timeline_panel = use_signal(|| TimelinePanel::new());
    let mut evidence_panel = use_signal(|| EvidencePanel::new());

    // Command palette state
    let mut show_command_palette = use_signal(|| false);
    let mut command_palette = use_signal(CommandPalette::new);

    // Tabbed settings
    let mut settings_tab = use_signal(|| "general".to_string());

    // Search state
    let mut search_query = use_signal(|| String::new());
    let mut show_search = use_signal(|| false);

    // User name for greeting
    let user_name = onboarding.read().user_name.clone().unwrap_or_default();
    let greeting = if user_name.is_empty() {
        "Hi there!".to_string()
    } else {
        format!("Hi {}!", user_name)
    };

    let mut send_message = move |text: String| {
        // Validate input
        let validation = validate_input(&text, 10_000);
        if !validation.valid {
            input_error.set(validation.error);
            return;
        }
        input_error.set(None);
        let text = validation.trimmed;

        messages.write().push(("user".into(), text.clone(), "message user".into()));
        connected.set(true);
        input.set(String::new());

        // Add to sidebar
        let task_id = format!("task-{}", messages.read().len());
        sidebar.write().add_task(&task_id, &text);

        // Show typing indicator
        is_typing.set(true);

        // Start async cognitive loop — calls real LLM API with sister integration
        let anthropic_key_val = settings_anthropic_key.read().clone();
        let openai_key_val = settings_openai_key.read().clone();
        let google_key_val = settings_google_key.read().clone();
        let model_val = settings_model.read().clone();
        let user_name = onboarding.read().user_name.clone().unwrap_or_default();
        let sisters_handle = sisters.read().clone();

        // Build conversation history for context
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
        };

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CognitiveUpdate>();

        // Spawn the cognitive loop (sends updates via channel)
        spawn(async move {
            run_cognitive_loop(loop_config, sisters_handle, tx).await;
        });

        // Spawn receiver coroutine that dispatches updates to UI signals
        spawn(async move {
            while let Some(update) = rx.recv().await {
                match update {
                    CognitiveUpdate::Phase(p) => phase.set(p),
                    CognitiveUpdate::IconState(s) => icon_state.set(s),
                    CognitiveUpdate::PhaseStatuses(s) => phase_statuses.set(s),
                    CognitiveUpdate::Typing(t) => is_typing.set(t),
                    CognitiveUpdate::PlanInit { goal, steps } => {
                        let step_refs: Vec<&str> = steps.iter().map(|s| s.as_str()).collect();
                        let mut pp = plan_panel.write();
                        *pp = PlanPanel::new(&goal, step_refs);
                    }
                    CognitiveUpdate::PlanClear => {
                        plan_panel.write().steps.clear();
                    }
                    CognitiveUpdate::PlanStepStart(idx) => {
                        plan_panel.write().start_step(idx);
                    }
                    CognitiveUpdate::PlanStepComplete { index, duration_ms } => {
                        if index == usize::MAX {
                            // Complete the last step
                            let step_count = plan_panel.read().steps.len();
                            if step_count > 0 {
                                plan_panel.write().complete_step(step_count - 1, None, duration_ms);
                            }
                        } else {
                            plan_panel.write().complete_step(index, None, duration_ms);
                        }
                    }
                    CognitiveUpdate::EvidenceClear => {
                        evidence_panel.write().clear();
                    }
                    CognitiveUpdate::EvidenceMemory { title, content } => {
                        evidence_panel.write().add_memory_context(&title, &content);
                    }
                    CognitiveUpdate::EvidenceCode { title, content, language, file_path } => {
                        evidence_panel.write().add_code(
                            &title, &content,
                            language.as_deref(), file_path.as_deref(), None,
                        );
                    }
                    CognitiveUpdate::TimelineClear => {
                        timeline_panel.write().clear();
                    }
                    CognitiveUpdate::Message { role, content, css_class } => {
                        messages.write().push((role, content, css_class));
                    }
                    CognitiveUpdate::SidebarCompleteTask(id) => {
                        sidebar.write().complete_task(&id);
                    }
                    CognitiveUpdate::Celebrate(msg) => {
                        celebration.set(Some(Celebration::small(&msg)));
                    }
                    CognitiveUpdate::ResetIdle => {
                        phase.set("Idle".into());
                        icon_state.set("idle".into());
                        active_progress.set(None);
                        phase_statuses.set(vec![]);
                    }
                    CognitiveUpdate::SuggestMode(mode) => {
                        // Step 4.7: Auto-select mode based on complexity
                        current_mode.set(mode);
                    }
                    CognitiveUpdate::AwaitApproval { risk_level, action, description, challenge_phrase } => {
                        // Step 3.7: Show approval card in UI
                        let card = match risk_level.as_str() {
                            "critical" => ApprovalCard::critical(&action, &description, challenge_phrase.as_deref().unwrap_or("")),
                            "high" => ApprovalCard::high(&action, &description, &action),
                            "medium" => ApprovalCard::medium(&action, &description),
                            _ => ApprovalCard::low(&action, &description),
                        };
                        pending_approval.set(Some(card));
                    }
                    CognitiveUpdate::SettingsApplied { confirmation } => {
                        // Step 4.9: Natural language settings applied
                        let _ = confirmation;
                    }
                    CognitiveUpdate::SistersCalled { sisters } => {
                        // Step 4.8: Track which sisters were used
                        let _ = sisters;
                    }
                    CognitiveUpdate::TokenUsage { input_tokens, output_tokens } => {
                        // Step 3.10: Token budget tracking
                        let _ = (input_tokens, output_tokens);
                    }
                    CognitiveUpdate::StreamChunk { content } => {
                        // Step 4.2: Append streaming token to current message
                        let _ = content;
                    }
                    CognitiveUpdate::StreamComplete => {
                        // Step 4.2: Streaming complete
                    }
                }
            }
        });
    };

    rsx! {
        style { {CSS} }

        // Apply theme to document root
        {
            let theme = settings_theme.read().clone();
            rsx! {
                script { "document.documentElement.setAttribute('data-theme', '{theme}')" }
            }
        }

        // Global keyboard handler
        div {
            class: "app-shell-wrapper",
            tabindex: "0",
            autofocus: true,
            onkeydown: move |e: KeyboardEvent| {
                let key = e.key();
                let meta = e.modifiers().contains(Modifiers::META);
                let _shift = e.modifiers().contains(Modifiers::SHIFT);

                if meta {
                    match key {
                        Key::Character(ref c) if c == "k" || c == "K" => {
                            e.prevent_default();
                            let current = *show_command_palette.read();
                            show_command_palette.set(!current);
                            if !current { command_palette.write().reset(); }
                        }
                        Key::Character(ref c) if c == "b" || c == "B" => {
                            e.prevent_default();
                            let current = *show_sidebar.read();
                            show_sidebar.set(!current);
                        }
                        Key::Character(ref c) if c == "," => {
                            e.prevent_default();
                            show_settings.set(true);
                        }
                        Key::Character(ref c) if c == "n" || c == "N" => {
                            e.prevent_default();
                            messages.write().clear();
                            phase.set("Idle".into());
                            icon_state.set("idle".into());
                        }
                        Key::Character(ref c) if c == "1" => { current_mode.set("companion".into()); }
                        Key::Character(ref c) if c == "2" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                        Key::Character(ref c) if c == "3" => { current_mode.set("immersive".into()); }
                        Key::Character(ref c) if c == "4" => { current_mode.set("invisible".into()); }
                        _ => {}
                    }
                }
                if key == Key::Escape {
                    show_command_palette.set(false);
                    show_settings.set(false);
                    show_features.set(false);
                    show_search.set(false);
                }
            },

            // ── Onboarding overlay ──
            if *show_onboarding.read() {
                div {
                    class: "onboarding-overlay",
                    div {
                        class: "onboarding-card",
                        // Globe
                        div {
                            class: "onboarding-globe",
                            div { class: "globe-core onboarding-glow" }
                        }
                        // Content based on step
                        {
                            let ob = onboarding.read();
                            let view = ob.current_view();
                            rsx! {
                                h2 { class: "onboarding-title", "{view.title}" }
                                p { class: "onboarding-subtitle", "{view.subtitle}" }
                            }
                        }
                        // Step-specific input
                        match onboarding.read().step {
                            OnboardingStep::Intro => rsx! {
                                p { class: "onboarding-subtitle", "I help with tasks, remember things, and keep you organized." }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        onboarding.write().advance();
                                    },
                                    "Continue"
                                }
                            },
                            OnboardingStep::AskName => rsx! {
                                input {
                                    class: "onboarding-input",
                                    placeholder: "Your name...",
                                    value: "{input}",
                                    oninput: move |e| input.set(e.value()),
                                    onkeypress: move |e| {
                                        if e.key() == Key::Enter && !input.read().trim().is_empty() {
                                            onboarding.write().set_name(input.read().trim());
                                            onboarding.write().advance();
                                            input.set(String::new());
                                        }
                                    }
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        if !input.read().trim().is_empty() {
                                            onboarding.write().set_name(input.read().trim());
                                            onboarding.write().advance();
                                            input.set(String::new());
                                        }
                                    },
                                    "Continue"
                                }
                            },
                            OnboardingStep::AskVoice => rsx! {
                                div {
                                    class: "onboarding-buttons",
                                    button {
                                        class: "btn-primary",
                                        onclick: move |_| {
                                            onboarding.write().enable_voice();
                                            onboarding.write().advance();
                                        },
                                        "Yes, enable"
                                    }
                                    button {
                                        class: "btn-secondary",
                                        onclick: move |_| {
                                            onboarding.write().advance();
                                        },
                                        "Maybe later"
                                    }
                                }
                            },
                            OnboardingStep::Complete => rsx! {
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        // Persist profile on completion
                                        let ob = onboarding.read();
                                        let profile = PersistedProfile {
                                            user_name: ob.user_name.clone(),
                                            voice_enabled: ob.voice_enabled,
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
                                        };
                                        save_profile(&profile);
                                        drop(ob);
                                        show_onboarding.set(false);
                                    },
                                    "Got it!"
                                }
                            },
                        }
                        // Step dots
                        div {
                            class: "step-dots",
                            {
                                let step = onboarding.read().step;
                                let c0: &str = if step == OnboardingStep::Intro { "step-dot active" } else { "step-dot done" };
                                let c1: &str = match step {
                                    OnboardingStep::Intro => "step-dot",
                                    OnboardingStep::AskName => "step-dot active",
                                    _ => "step-dot done",
                                };
                                let c2: &str = match step {
                                    OnboardingStep::AskVoice => "step-dot active",
                                    OnboardingStep::Complete => "step-dot done",
                                    _ => "step-dot",
                                };
                                let c3: &str = if step == OnboardingStep::Complete { "step-dot active" } else { "step-dot" };
                                rsx! {
                                    div { class: c0 }
                                    div { class: c1 }
                                    div { class: c2 }
                                    div { class: c3 }
                                }
                            }
                        }
                    }
                }
            }

            // ── Command Palette overlay ──
            if *show_command_palette.read() {
                div {
                    class: "command-palette-overlay",
                    onclick: move |_| show_command_palette.set(false),
                    div {
                        class: "command-palette",
                        onclick: move |e: MouseEvent| e.stop_propagation(),
                        input {
                            class: "command-palette-input",
                            placeholder: "Type a command...",
                            value: "{command_palette.read().query}",
                            autofocus: true,
                            oninput: move |e| command_palette.write().set_query(&e.value()),
                            onkeydown: move |e| {
                                match e.key() {
                                    Key::ArrowUp => { e.prevent_default(); command_palette.write().select_up(); }
                                    Key::ArrowDown => { e.prevent_default(); command_palette.write().select_down(); }
                                    Key::Enter => {
                                        e.prevent_default();
                                        let id = command_palette.read().selected_command_id();
                                        if let Some(cmd_id) = id {
                                            command_palette.write().record_usage(&cmd_id);
                                            show_command_palette.set(false);
                                            // Handle command
                                            match cmd_id.as_str() {
                                                "toggle-sidebar" => { let c = *show_sidebar.read(); show_sidebar.set(!c); }
                                                "open-settings" => show_settings.set(true),
                                                "mode-companion" => current_mode.set("companion".into()),
                                                "mode-workspace" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                                                "mode-immersive" => current_mode.set("immersive".into()),
                                                "mode-invisible" => current_mode.set("invisible".into()),
                                                "clear-chat" => messages.write().clear(),
                                                "view-features" => show_features.set(true),
                                                "new-session" => {
                                                    messages.write().clear();
                                                    phase.set("Idle".into());
                                                    icon_state.set("idle".into());
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    Key::Escape => show_command_palette.set(false),
                                    _ => {}
                                }
                            },
                        }
                        div {
                            class: "command-palette-results",
                            {
                                let cp = command_palette.read();
                                let filtered = cp.filtered();
                                let selected = cp.selected_index;
                                rsx! {
                                    for (idx, cmd) in filtered.iter().enumerate() {
                                        div {
                                            class: if idx == selected { "command-item selected" } else { "command-item" },
                                            span { class: "command-label", "{cmd.label}" }
                                            if let Some(ref shortcut) = cmd.shortcut {
                                                span { class: "command-shortcut", "{shortcut}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Features overlay ──
            if *show_features.read() {
                div {
                    class: "settings-overlay",
                    div {
                        class: "settings-card features-card",
                        h2 { class: "onboarding-title", "Features & Capabilities" }
                        p { class: "features-subtitle", "All Hydra systems and inventions" }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Cognitive Inventions" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Dream State" }
                                div { class: "feature-chip active", "Shadow Self" }
                                div { class: "feature-chip active", "Resurrection" }
                                div { class: "feature-chip active", "Token Minimizer" }
                                div { class: "feature-chip active", "Future Echo" }
                                div { class: "feature-chip active", "Mutation Engine" }
                                div { class: "feature-chip active", "Forking" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Agent Swarm & Federation" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Peer Discovery" }
                                div { class: "feature-chip active", "Task Delegation" }
                                div { class: "feature-chip active", "Skill Sharing" }
                                div { class: "feature-chip active", "Load Balancing" }
                                div { class: "feature-chip", "Multi-Instance Sync" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Model & Intelligence" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Model Router" }
                                div { class: "feature-chip active", "Circuit Breaker" }
                                div { class: "feature-chip active", "Intent Compiler" }
                                div { class: "feature-chip active", "Belief Revision" }
                                div { class: "feature-chip active", "Pattern Detection" }
                                div { class: "feature-chip active", "Animus Prime" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Safety & Control" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Execution Gate" }
                                div { class: "feature-chip active", "Kill Switch" }
                                div { class: "feature-chip active", "Risk Assessment" }
                                div { class: "feature-chip active", "Boundary Enforcer" }
                                div { class: "feature-chip active", "Challenge Phrases" }
                                div { class: "feature-chip active", "Harm Prediction" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Sisters & Skills" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Memory Bridge" }
                                div { class: "feature-chip active", "Vision Bridge" }
                                div { class: "feature-chip active", "Codebase Bridge" }
                                div { class: "feature-chip active", "Identity Bridge" }
                                div { class: "feature-chip active", "Cognition Bridge" }
                                div { class: "feature-chip active", "Skill Sandbox" }
                                div { class: "feature-chip active", "Skill Registry" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Voice & Interaction" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip", "Speech-to-Text" }
                                div { class: "feature-chip", "Text-to-Speech" }
                                div { class: "feature-chip", "Wake Word" }
                                div { class: "feature-chip active", "Voice Commands" }
                                div { class: "feature-chip active", "Pulse Engine" }
                                div { class: "feature-chip active", "Proactive Alerts" }
                            }
                        }

                        div {
                            class: "feature-section",
                            h3 { class: "feature-section-title", "Observability" }
                            div {
                                class: "feature-grid",
                                div { class: "feature-chip active", "Structured Logging" }
                                div { class: "feature-chip active", "Metrics" }
                                div { class: "feature-chip active", "Distributed Tracing" }
                                div { class: "feature-chip active", "Receipt Ledger" }
                                div { class: "feature-chip active", "Protocol Hunter" }
                            }
                        }

                        button {
                            class: "btn-primary",
                            style: "margin-top: 12px; width: 100%;",
                            onclick: move |_| show_features.set(false),
                            "Close"
                        }
                    }
                }
            }

            // ── Main app shell ──
            div {
                class: if *show_sidebar.read() { "app-shell with-sidebar" } else { "app-shell" },

                // ── Sidebar ──
                if *show_sidebar.read() {
                    div {
                        class: "sidebar",
                        // Sidebar header
                        div {
                            class: "sidebar-header",
                            span { class: "sidebar-brand", "Hydra" }
                            button {
                                class: "sidebar-new-btn",
                                title: "New Session (Cmd+N)",
                                onclick: move |_| {
                                    messages.write().clear();
                                    phase.set("Idle".into());
                                    icon_state.set("idle".into());
                                },
                                "+"
                            }
                        }

                        // Session list — render today's tasks
                        div {
                            class: "sidebar-sessions",
                            div {
                                class: "sidebar-section",
                                div {
                                    class: "sidebar-section-title",
                                    onclick: move |_| sidebar.write().toggle_section(0),
                                    span { "Today" }
                                    {
                                        let ch = if sidebar.read().sections.first().map_or(false, |s| s.collapsed) { "\u{25B8}" } else { "\u{25BE}" };
                                        rsx! { span { class: "sidebar-chevron", "{ch}" } }
                                    }
                                }
                                if !sidebar.read().sections.first().map_or(true, |s| s.collapsed) {
                                    for item in sidebar.read().today_items().iter() {
                                        div {
                                            class: if item.active { "sidebar-item active" } else { "sidebar-item" },
                                            span {
                                                class: {
                                                    let c = if item.active { "sidebar-dot active" } else if item.icon == "\u{2713}" { "sidebar-dot done" } else { "sidebar-dot" };
                                                    c.to_string()
                                                },
                                            }
                                            span { class: "sidebar-item-label", "{item.label}" }
                                        }
                                    }
                                }
                            }
                        }

                        // Sidebar footer
                        div {
                            class: "sidebar-footer",
                            button {
                                class: "sidebar-settings-btn",
                                onclick: move |_| {
                                    let current = *show_settings.read();
                                    show_settings.set(!current);
                                },
                                title: "Settings (Cmd+,)",
                                "\u{2699}"
                            }
                            div {
                                class: "sidebar-status",
                                div {
                                    class: if *connected.read() { "status-dot connected" } else { "status-dot" },
                                }
                                span { class: "sidebar-status-text", "{sisters_status}" }
                            }
                        }
                    }
                }

                // ── Main content area ──
                div {
                    class: "main-content",

                    // ── TopBar ──
                    div {
                        class: "topbar",
                        div {
                            class: "topbar-left",
                            // Show brand only when sidebar is hidden
                            if !*show_sidebar.read() {
                                span { class: "topbar-brand", "Hydra" }
                            }
                            // Mode indicator
                            span { class: "topbar-mode", "{current_mode}" }
                        }
                        div {
                            class: "topbar-center",
                            // Phase dots
                            {
                                let statuses = phase_statuses.read();
                                if !statuses.is_empty() {
                                    let dots = build_phase_dots(&statuses);
                                    let connectors = build_connectors(&statuses);
                                    rsx! {
                                        div {
                                            class: "phase-dots",
                                            for (idx, dot) in dots.iter().enumerate() {
                                                if idx > 0 {
                                                    {
                                                        let conn = &connectors[idx - 1];
                                                        rsx! {
                                                            div {
                                                                class: if conn.active { "phase-connector active" } else { "phase-connector" },
                                                            }
                                                        }
                                                    }
                                                }
                                                div {
                                                    title: "{dot.label}",
                                                    class: format!("phase-dot {}", dot.css_class),
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }
                            // Phase label
                            {
                                let p = phase.read();
                                let label = match p.as_str() {
                                    "Perceive" | "Think" | "Decide" | "Act" => "Working...",
                                    "Learn" => "Finishing up...",
                                    "Done" => "Done",
                                    "Error" => "Error",
                                    "Idle" => "",
                                    other => other,
                                };
                                if !label.is_empty() {
                                    rsx! { span { class: "topbar-phase-label", "{label}" } }
                                } else {
                                    rsx! {}
                                }
                            }
                        }
                        div {
                            class: "topbar-right",
                            // Command palette shortcut hint
                            button {
                                class: "topbar-cmd-btn",
                                title: "Command Palette (Cmd+K)",
                                onclick: move |_| {
                                    command_palette.write().reset();
                                    show_command_palette.set(true);
                                },
                                "\u{2318}K"
                            }
                            // Toggle sidebar
                            button {
                                class: "topbar-icon-btn",
                                title: "Toggle Sidebar (Cmd+B)",
                                onclick: move |_| {
                                    let current = *show_sidebar.read();
                                    show_sidebar.set(!current);
                                },
                                "\u{2630}"
                            }
                            // Settings
                            button {
                                class: "topbar-icon-btn",
                                title: "Settings (Cmd+,)",
                                onclick: move |_| {
                                    let current = *show_settings.read();
                                    show_settings.set(!current);
                                },
                                "\u{2699}"
                            }
                        }
                    }

                    // ── Content: Settings page OR Chat ──
                    if *show_settings.read() {
                        // Full-page settings with left tabs
                        div {
                            class: "settings-page",
                            div {
                                class: "settings-tabs",
                                {
                                    let tabs: Vec<(&str, &str)> = vec![
                                        ("general", "General"),
                                        ("models", "Models"),
                                        ("sisters", "Sisters"),
                                        ("voice", "Voice"),
                                        ("policies", "Policies"),
                                        ("advanced", "Advanced"),
                                    ];
                                    let current_tab = settings_tab.read().clone();
                                    rsx! {
                                        for (id, label) in tabs.iter() {
                                            button {
                                                class: if current_tab == *id { "settings-tab active" } else { "settings-tab" },
                                                onclick: {
                                                    let tab_id = id.to_string();
                                                    move |_| settings_tab.set(tab_id.clone())
                                                },
                                                "{label}"
                                            }
                                        }
                                    }
                                }
                            }
                            div {
                                class: "settings-content",
                                // Tab content
                                match settings_tab.read().as_str() {
                                    "general" => rsx! {
                                        h2 { class: "settings-page-title", "General" }
                                        div {
                                            class: "settings-section",
                                            h3 { class: "settings-section-title", "Appearance" }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Theme" }
                                                div { class: "settings-select",
                                                    button {
                                                        class: if *settings_theme.read() == "dark" { "settings-option active" } else { "settings-option" },
                                                        onclick: move |_| settings_theme.set("dark".into()),
                                                        "Dark"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "light" { "settings-option active" } else { "settings-option" },
                                                        onclick: move |_| settings_theme.set("light".into()),
                                                        "Light"
                                                    }
                                                    button {
                                                        class: if *settings_theme.read() == "system" { "settings-option active" } else { "settings-option" },
                                                        onclick: move |_| settings_theme.set("system".into()),
                                                        "System"
                                                    }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Default Mode" }
                                                div { class: "settings-select",
                                                    button {
                                                        class: if *settings_default_mode.read() == "companion" { "settings-option active" } else { "settings-option" },
                                                        onclick: move |_| settings_default_mode.set("companion".into()),
                                                        "Companion"
                                                    }
                                                    button {
                                                        class: if *settings_default_mode.read() == "workspace" { "settings-option active" } else { "settings-option" },
                                                        onclick: move |_| settings_default_mode.set("workspace".into()),
                                                        "Workspace"
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "models" => rsx! {
                                        h2 { class: "settings-page-title", "Models" }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Anthropic" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("claude-sonnet-4-6", "Sonnet 4.6"), ("claude-opus-4-6", "Opus 4.6"), ("claude-haiku-4-5", "Haiku 4.5")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "settings-option active" } else { "settings-option" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "settings-key-row",
                                                input {
                                                    class: "settings-input",
                                                    r#type: "password",
                                                    placeholder: "Anthropic API Key (sk-ant-...)",
                                                    value: "{settings_anthropic_key}",
                                                    oninput: move |e| settings_anthropic_key.set(e.value()),
                                                }
                                                if !settings_anthropic_key.read().is_empty() {
                                                    span { class: "settings-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "OpenAI" }
                                            div { class: "model-grid",
                                                {
                                                    let models: Vec<(&str, &str)> = vec![("gpt-4o", "GPT-4o"), ("gpt-4o-mini", "GPT-4o Mini")];
                                                    rsx! {
                                                        for (id, label) in models.iter() {
                                                            button {
                                                                class: if *settings_model.read() == *id { "settings-option active" } else { "settings-option" },
                                                                onclick: { let m = id.to_string(); move |_| settings_model.set(m.clone()) },
                                                                "{label}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            div { class: "settings-key-row",
                                                input {
                                                    class: "settings-input",
                                                    r#type: "password",
                                                    placeholder: "OpenAI API Key (sk-...)",
                                                    value: "{settings_openai_key}",
                                                    oninput: move |e| settings_openai_key.set(e.value()),
                                                }
                                                if !settings_openai_key.read().is_empty() {
                                                    span { class: "settings-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Google" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "gemini-2.0-flash" { "settings-option active" } else { "settings-option" },
                                                    onclick: move |_| settings_model.set("gemini-2.0-flash".into()),
                                                    "Gemini Flash"
                                                }
                                            }
                                            div { class: "settings-key-row",
                                                input {
                                                    class: "settings-input",
                                                    r#type: "password",
                                                    placeholder: "Google API Key",
                                                    value: "{settings_google_key}",
                                                    oninput: move |e| settings_google_key.set(e.value()),
                                                }
                                                if !settings_google_key.read().is_empty() {
                                                    span { class: "settings-check", "\u{2713}" }
                                                }
                                            }
                                        }
                                        div { class: "settings-section",
                                            h3 { class: "settings-section-title", "Local" }
                                            div { class: "model-grid",
                                                button {
                                                    class: if *settings_model.read() == "ollama" { "settings-option active" } else { "settings-option" },
                                                    onclick: move |_| settings_model.set("ollama".into()),
                                                    "Ollama"
                                                }
                                            }
                                            span { class: "settings-info", "No key needed \u{2014} runs locally" }
                                        }
                                        div { class: "settings-info", style: "margin-top: 16px;",
                                            "Keys saved to ~/.hydra/profile.json. Also auto-detected from environment variables."
                                        }
                                    },
                                    "sisters" => rsx! {
                                        h2 { class: "settings-page-title", "Sisters & MCP" }
                                        div { class: "settings-section",
                                            p { class: "settings-info", style: "margin-bottom: 16px;",
                                                "Hydra connects to 14 sister agents via MCP (Model Context Protocol)."
                                            }
                                            div {
                                                class: "settings-mcp-status",
                                                style: "color: var(--accent); margin-bottom: 16px;",
                                                "{sisters_status}"
                                            }
                                            {
                                                let sh = sisters.read();
                                                let mem_connected = sh.as_ref().map_or(false, |s| s.memory.is_some());
                                                let id_connected = sh.as_ref().map_or(false, |s| s.identity.is_some());
                                                let cb_connected = sh.as_ref().map_or(false, |s| s.codebase.is_some());
                                                let vis_connected = sh.as_ref().map_or(false, |s| s.vision.is_some());
                                                let mem_tools = sh.as_ref().and_then(|s| s.memory.as_ref()).map(|m| m.tools.len()).unwrap_or(0);
                                                let id_tools = sh.as_ref().and_then(|s| s.identity.as_ref()).map(|m| m.tools.len()).unwrap_or(0);
                                                let cb_tools = sh.as_ref().and_then(|s| s.codebase.as_ref()).map(|m| m.tools.len()).unwrap_or(0);
                                                let vis_tools = sh.as_ref().and_then(|s| s.vision.as_ref()).map(|m| m.tools.len()).unwrap_or(0);
                                                let total_tools = mem_tools + id_tools + cb_tools + vis_tools;
                                                let sister_list: Vec<(&str, bool, usize)> = vec![
                                                    ("Memory", mem_connected, mem_tools),
                                                    ("Identity", id_connected, id_tools),
                                                    ("Codebase", cb_connected, cb_tools),
                                                    ("Vision", vis_connected, vis_tools),
                                                ];
                                                rsx! {
                                                    div { class: "sisters-total",
                                                        "{total_tools} tools connected"
                                                    }
                                                    div { class: "sisters-grid",
                                                        for (name, conn, tools) in sister_list.iter() {
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
                                                }
                                            }
                                        }
                                    },
                                    "voice" => rsx! {
                                        h2 { class: "settings-page-title", "Voice & Audio" }
                                        div { class: "settings-section",
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Voice Mode" }
                                                button {
                                                    class: if *settings_voice.read() { "settings-toggle on" } else { "settings-toggle" },
                                                    onclick: move |_| { let c = *settings_voice.read(); settings_voice.set(!c); },
                                                    { let t = if *settings_voice.read() { "Enabled" } else { "Disabled" }; rsx! { "{t}" } }
                                                }
                                            }
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Sound Effects" }
                                                button {
                                                    class: if *settings_sounds.read() { "settings-toggle on" } else { "settings-toggle" },
                                                    onclick: move |_| { let c = *settings_sounds.read(); settings_sounds.set(!c); },
                                                    { let t = if *settings_sounds.read() { "Enabled" } else { "Disabled" }; rsx! { "{t}" } }
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
                                            p { class: "settings-info",
                                                "STT: Whisper (local). TTS: Piper (local). Wake word detection supported."
                                            }
                                        }
                                    },
                                    "policies" => rsx! {
                                        h2 { class: "settings-page-title", "Safety & Policies" }
                                        div { class: "settings-section",
                                            div { class: "settings-row",
                                                span { class: "settings-label", "Auto-approve low-risk actions" }
                                                button {
                                                    class: if *settings_auto_approve.read() { "settings-toggle on" } else { "settings-toggle" },
                                                    onclick: move |_| { let c = *settings_auto_approve.read(); settings_auto_approve.set(!c); },
                                                    { let t = if *settings_auto_approve.read() { "Enabled" } else { "Disabled" }; rsx! { "{t}" } }
                                                }
                                            }
                                            p { class: "settings-info",
                                                "Execution Gate evaluates risk before every action. Kill Switch provides emergency stop. Boundary Enforcer sets hard limits."
                                            }
                                        }
                                    },
                                    _ => rsx! {
                                        h2 { class: "settings-page-title", "Advanced" }
                                        div { class: "settings-section",
                                            p { class: "settings-info", "Server: http://127.0.0.1:3100 | SSE: /events | RPC: /rpc" }
                                            p { class: "settings-info", "Config: ~/.hydra/config.toml | Profile: ~/.hydra/profile.json" }
                                            p { class: "settings-info", "Database: ~/.hydra/hydra.db | MCP: ~/.hydra/mcp.json" }
                                        }
                                    },
                                }
                                // Save button at bottom of every tab
                                div { class: "settings-save-area",
                                    button {
                                        class: "btn-primary",
                                        onclick: move |_| {
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
                                            };
                                            save_profile(&profile);

                                            let mode = settings_default_mode.read().clone();
                                            current_mode.set(mode.clone());
                                            show_sidebar.set(mode == "workspace");

                                            show_settings.set(false);
                                        },
                                        "Save & Close"
                                    }
                                }
                            }
                        }
                    } else if *current_mode.read() == "invisible" {
                        // Invisible mode
                        div {
                            class: "invisible-mode",
                            div {
                                class: format!("globe-container icon-{}", icon_state.read()),
                                div { class: "globe",
                                    div { class: "globe-core" }
                                }
                            }
                            p { class: "invisible-hint", "Say \"Hey Hydra\" or press Cmd+1 to switch modes" }
                        }
                    } else {
                        // ── Chat view ──
                        div {
                            class: "chat-container",

                            // Workspace panels (only in workspace/immersive mode with multi-step plan)
                            if (*current_mode.read() == "workspace" || *current_mode.read() == "immersive") && plan_panel.read().steps.len() > 1 {
                                div {
                                    class: "workspace-panels",

                                    // Plan Panel
                                    div {
                                        class: "panel plan-panel",
                                        h3 { class: "panel-title", "Plan" }
                                        {
                                            let pp = plan_panel.read();
                                            rsx! {
                                                div { class: "plan-goal", "{pp.goal}" }
                                                div { class: "plan-steps",
                                                    for step in pp.steps.iter() {
                                                        div {
                                                            class: format!("plan-step {}", match step.status {
                                                                StepStatus::Completed => "completed",
                                                                StepStatus::Running => "running",
                                                                StepStatus::Failed => "failed",
                                                                StepStatus::Skipped => "skipped",
                                                                StepStatus::Pending => "pending",
                                                            }),
                                                            span { class: "step-icon",
                                                                "{PlanPanel::step_icon(step.status)}"
                                                            }
                                                            span { class: "step-label", "{step.label}" }
                                                        }
                                                    }
                                                }
                                                div { class: "plan-progress",
                                                    div { class: "progress-bar",
                                                        div {
                                                            class: "progress-fill",
                                                            style: format!("width: {}%", pp.progress_percent()),
                                                        }
                                                    }
                                                    span { class: "progress-text", "{pp.progress_percent() as u32}%" }
                                                }
                                                if let Some(eta) = pp.eta_display() {
                                                    span { class: "plan-eta", "ETA: {eta}" }
                                                }
                                            }
                                        }
                                    }

                                    // Timeline Panel
                                    {
                                        let tp = timeline_panel.read();
                                        let user_events: Vec<_> = tp.events.iter()
                                            .filter(|e| e.kind != TimelineEventKind::PhaseChange)
                                            .collect();
                                        if !user_events.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel timeline-panel",
                                                    h3 { class: "panel-title", "Timeline" }
                                                    div { class: "timeline-events",
                                                        for event in user_events.iter() {
                                                            div {
                                                                class: format!("timeline-event {}", TimelinePanel::event_css_class(event.kind)),
                                                                span { class: "timeline-icon", "{TimelinePanel::event_icon(event.kind)}" }
                                                                span { class: "timeline-time", "{event.timestamp}" }
                                                                span { class: "timeline-label", "{event.title}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }

                                    // Evidence Panel
                                    {
                                        let ep = evidence_panel.read();
                                        let meaningful_items: Vec<_> = ep.items.iter()
                                            .filter(|item| EvidencePanel::is_meaningful(item))
                                            .collect();
                                        if !meaningful_items.is_empty() {
                                            rsx! {
                                                div {
                                                    class: "panel evidence-panel",
                                                    h3 { class: "panel-title", "Evidence" }
                                                    div { class: "evidence-items",
                                                        for item in meaningful_items.iter() {
                                                            div {
                                                                class: format!("evidence-item {}", EvidencePanel::evidence_css_class(item.kind)),
                                                                div { class: "evidence-header",
                                                                    span { class: "evidence-icon", "{EvidencePanel::evidence_icon(item.kind)}" }
                                                                    span { class: "evidence-title", "{item.title}" }
                                                                    if item.pinned {
                                                                        span { class: "evidence-pin", "\u{1F4CC}" }
                                                                    }
                                                                }
                                                                {
                                                                    let summary = EvidencePanel::human_summary(item);
                                                                    rsx! {
                                                                        p { class: "evidence-content", "{summary}" }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            rsx! {}
                                        }
                                    }
                                }
                            }

                            // Messages area
                            div {
                                class: "messages-list",
                                id: "messages-container",

                                // Welcome state (no messages)
                                if messages.read().is_empty() {
                                    div {
                                        class: "welcome-state",
                                        div {
                                            class: format!("globe-container icon-{}", icon_state.read()),
                                            div { class: "globe",
                                                div { class: "globe-ring ring-1" }
                                                div { class: "globe-ring ring-2" }
                                                div { class: "globe-core" }
                                            }
                                        }
                                        h2 { class: "welcome-title", "{greeting}" }
                                        p { class: "welcome-subtitle", "How can I help you today?" }
                                    }
                                }

                                // Message list
                                for (i, (role, content, _css)) in messages.read().iter().enumerate() {
                                    div {
                                        key: "{i}",
                                        class: if role == "user" { "message message-user" } else { "message message-hydra" },
                                        {
                                            let role_label = if role == "user" { "You" } else { "Hydra" };
                                            rsx! { div { class: "message-role", "{role_label}" } }
                                        }
                                        div {
                                            class: "message-content",
                                            dangerous_inner_html: markdown_to_html(content),
                                        }
                                    }
                                }

                                // Typing indicator
                                if *is_typing.read() {
                                    div {
                                        class: "typing-indicator",
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                        div { class: "typing-dot" }
                                    }
                                }
                            }

                            // Auto-scroll
                            {
                                let _count = messages.read().len();
                                let _typing = *is_typing.read();
                                rsx! { script { "requestAnimationFrame(function(){{ var el = document.getElementById('messages-container'); if(el) el.scrollTop = el.scrollHeight; }})" } }
                            }

                            // Approval card (if pending)
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
                                    rsx! {
                                        div {
                                            class: "approval-card",
                                            tabindex: "0",
                                            onkeydown: move |e| {
                                                match e.key() {
                                                    Key::Character(ref c) if c == "y" || c == "Y" => {
                                                        pending_approval.set(None);
                                                        approval_countdown.set(0);
                                                    }
                                                    Key::Character(ref c) if c == "n" || c == "N" => {
                                                        pending_approval.set(None);
                                                        approval_countdown.set(0);
                                                    }
                                                    _ => {}
                                                }
                                            },
                                            div {
                                                class: "approval-header",
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
                                                    class: "challenge-input",
                                                    value: "{challenge_input}",
                                                    oninput: move |e| challenge_input.set(e.value()),
                                                }
                                            }
                                            if countdown_val > 0 {
                                                div {
                                                    class: "approval-countdown",
                                                    "Auto-declining in {countdown_val}s"
                                                }
                                                div {
                                                    class: "approval-progress-bar",
                                                    div {
                                                        class: "approval-progress-fill",
                                                        style: format!("width: {}%", (countdown_val as f32 / 30.0 * 100.0).min(100.0)),
                                                    }
                                                }
                                            }
                                            div {
                                                class: "approval-actions",
                                                button {
                                                    class: "btn-primary",
                                                    onclick: move |_| {
                                                        pending_approval.set(None);
                                                        approval_countdown.set(0);
                                                    },
                                                    "{primary} "
                                                    span { class: "kbd", "Y" }
                                                }
                                                button {
                                                    class: "btn-secondary",
                                                    onclick: move |_| {
                                                        pending_approval.set(None);
                                                        approval_countdown.set(0);
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

                            // Celebration toast
                            {
                                let cel = celebration.read();
                                if let Some(c) = cel.as_ref() {
                                    let emoji = c.emoji.clone();
                                    let msg = c.message.clone();
                                    rsx! {
                                        div {
                                            class: "celebration-toast",
                                            onclick: move |_| celebration.set(None),
                                            span { class: "celebration-emoji", "{emoji}" }
                                            span { class: "celebration-msg", "{msg}" }
                                        }
                                    }
                                } else {
                                    rsx! {}
                                }
                            }

                            // Error display
                            {
                                let err = active_error.read();
                                if let Some(error) = err.as_ref() {
                                    let msg = error.message.clone();
                                    let expl = error.explanation.clone();
                                    let opts: Vec<(String, bool)> = error.options.iter()
                                        .map(|o| (o.label.clone(), o.is_primary))
                                        .collect();
                                    rsx! {
                                        div {
                                            class: "error-card",
                                            div { class: "error-icon", "\u{25C9}" }
                                            p { class: "error-message", "{msg}" }
                                            p { class: "error-explanation", "{expl}" }
                                            div {
                                                class: "error-options",
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
                                    input {
                                        class: "chat-input",
                                        placeholder: "Message Hydra...",
                                        value: "{input}",
                                        oninput: move |e| {
                                            input.set(e.value());
                                            if input_error.read().is_some() { input_error.set(None); }
                                        },
                                        onkeypress: move |e| {
                                            if e.key() == Key::Enter {
                                                let text = input.read().clone();
                                                send_message(text);
                                            }
                                        },
                                    }
                                    button {
                                        class: "send-btn",
                                        onclick: move |_| {
                                            let text = input.read().clone();
                                            send_message(text);
                                        },
                                        "\u{2191}"
                                    }
                                }
                                // Input error
                                {
                                    let err = input_error.read();
                                    if let Some(ref msg) = *err {
                                        rsx! {
                                            div { class: "input-error", "{msg}" }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                                p { class: "input-hint",
                                    "Enter to send \u{00B7} \u{2318}K commands \u{00B7} \u{2318}B sidebar"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Launch the Hydra desktop application via Dioxus.
pub fn launch() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Hydra")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
                        .with_min_inner_size(dioxus::desktop::LogicalSize::new(800.0, 600.0)),
                ),
        )
        .launch(App);
}
