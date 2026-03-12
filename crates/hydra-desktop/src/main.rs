//! Hydra Desktop — Fresh build. Claude Desktop quality.

use dioxus::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod voice_capture;

// Data structures from hydra-native (tested, working)
use hydra_native::components::onboarding::{OnboardingState, OnboardingStep};
use hydra_native::components::approval::ApprovalCard;
use hydra_native::components::progress::{Celebration, ProgressJourney};
use hydra_native::components::error_display::FriendlyError;
use hydra_native::components::sidebar::Sidebar;
use hydra_native::components::input::validate_input;
use hydra_native::components::phases::{build_phase_dots, build_connectors};
use hydra_native::components::command_palette::CommandPalette;
use hydra_native::state::hydra::PhaseStatus;
use hydra_native::components::plan_panel::{PlanPanel, StepStatus};
use hydra_native::components::timeline_panel::{TimelinePanel, TimelineEventKind};
use hydra_native::components::evidence_panel::EvidencePanel;
use hydra_native::profile::{load_profile, save_profile, PersistedProfile};
use hydra_native::sisters::SistersHandle;
use hydra_native::cognitive::{CognitiveLoopConfig, CognitiveUpdate, DecideEngine, InventionEngine, AgentSpawner, run_cognitive_loop};
use hydra_native::proactive::ProactiveNotifier;
use hydra_model::oauth::AnthropicOAuth;
use hydra_native::persistence::ChatPersistence;
use hydra_native::utils::markdown::markdown_to_html;
use hydra_native::components::globe::{globe_params, globe_svg, derive_globe_state, GlobeSize};
use hydra_native::components::ghost_cursor::{GhostCursorState, CursorMode, cursor_svg};
use hydra_db::HydraDb;
use hydra_runtime::approval::{ApprovalDecision, ApprovalManager};
use hydra_runtime::undo::UndoStack;
use hydra_native::federation::FederationManager;

const CSS: &str = include_str!("styles.css");

fn main() {
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

#[allow(non_snake_case)]
fn App() -> Element {
    // ── Per-project lock — prevents two Hydras on the same project ──
    let _project_lock: Arc<parking_lot::Mutex<hydra_runtime::InstanceLock>> = use_hook(|| {
        let project_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut lock = hydra_runtime::InstanceLock::for_project(&project_dir);
        if let Err(e) = lock.acquire() {
            eprintln!("[hydra] {}", e);
        }
        Arc::new(parking_lot::Mutex::new(lock))
    });

    // ── Load persisted profile ──
    let persisted = load_profile();
    let onboarding_done = persisted.as_ref().map_or(false, |p| p.onboarding_complete);

    let chat_db = Arc::new(ChatPersistence::init().unwrap_or_else(|e| {
        eprintln!("[hydra] Chat persistence failed: {}", e);
        ChatPersistence::init().expect("chat persistence fallback failed")
    }));

    // ── Init engines + security DB (extracted for compilation memory) ──
    let (decide_engine, invention_engine, proactive_notifier, agent_spawner, undo_stack, approval_manager, federation_manager, hydra_db) = include!("app_engines.rs");

    // Clone for each UI consumer that captures approval_manager
    let send_msg_approval_mgr = approval_manager.clone();
    let palette_approval_mgr = approval_manager.clone();
    let card_approval_mgr = approval_manager.clone();

    // ── Init sisters (MCP connections) ──
    let sisters: Signal<Option<SistersHandle>> = use_signal(|| None);
    let sisters_status = use_signal(|| "Connecting...".to_string());
    {
        let mut sisters = sisters.clone();
        let mut sisters_status = sisters_status.clone();
        use_hook(move || {
            spawn(async move {
                let handle = hydra_native::sisters::init_sisters().await;
                let status = handle.status_summary();
                sisters.set(Some(handle));
                sisters_status.set(status);
            });
        });
    }

    // ── Extract settings from profile ──
    let init_theme = persisted.as_ref().and_then(|p| p.theme.clone()).unwrap_or_else(|| "dark".to_string());
    let init_voice = persisted.as_ref().map_or(false, |p| p.voice_enabled);
    let init_sounds = persisted.as_ref().map_or(true, |p| p.sounds_enabled);
    let init_volume = persisted.as_ref().map_or("70".to_string(), |p| p.sound_volume.to_string());
    let init_auto_approve = persisted.as_ref().map_or(false, |p| p.auto_approve);
    let init_default_mode = persisted.as_ref().and_then(|p| p.default_mode.clone()).unwrap_or_else(|| "companion".to_string());
    let init_model = persisted.as_ref().and_then(|p| p.selected_model.clone()).unwrap_or_else(|| "claude-sonnet-4-6".to_string());
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

    // ── Core state signals ──
    let mut input = use_signal(|| String::new());
    let chat_db_init = chat_db.clone();
    let mut messages = use_signal(move || chat_db_init.load_messages());
    let chat_db_sig: Signal<Arc<ChatPersistence>> = use_signal(|| chat_db.clone());
    let mut connected = use_signal(|| false);
    let mut phase = use_signal(|| "Idle".to_string());
    let mut icon_state = use_signal(|| "idle".to_string());

    // ── Onboarding ──
    let mut onboarding = use_signal(move || {
        if let Some(ref p) = persisted {
            let mut state = OnboardingState::new();
            if let Some(ref name) = p.user_name { state.set_name(name); }
            if p.voice_enabled { state.enable_voice(); }
            if p.onboarding_complete { state.advance(); state.advance(); state.advance(); }
            state
        } else {
            OnboardingState::new()
        }
    });
    let mut show_onboarding = use_signal(move || !onboarding_done);

    // ── Settings state ──
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
    let mut settings_anthropic_key = use_signal(move || init_anthropic_key);
    let mut settings_openai_key = use_signal(move || init_openai_key);
    let mut settings_google_key = use_signal(move || init_google_key);
    let mut settings_tab = use_signal(|| "general".to_string());

    // ── Anthropic OAuth state ──
    let mut oauth_status = use_signal(|| {
        let oauth = AnthropicOAuth::new();
        if oauth.is_authenticated() {
            let email = oauth.account_email().unwrap_or("").to_string();
            let tier = oauth.subscription_tier().unwrap_or("").to_string();
            ("authenticated".to_string(), email, tier)
        } else {
            ("not_authenticated".to_string(), String::new(), String::new())
        }
    });
    let mut oauth_loading = use_signal(|| false);

    // ── Mode state ──
    let mut current_mode = use_signal(move || init_mode_for_current);

    // ── Sidebar ──
    let mut sidebar = use_signal(|| {
        let mut sb = Sidebar::new();
        sb.add_task("session-1", "Session 1");
        sb
    });
    let mut show_sidebar = use_signal(move || init_mode_for_sidebar == "workspace");

    // ── Approval, progress, error ──
    let mut pending_approval = use_signal(|| Option::<ApprovalCard>::None);
    let mut pending_approval_id = use_signal(|| Option::<String>::None);
    let mut challenge_input = use_signal(|| String::new());

    // ── Ghost Cursor state ──
    let mut ghost_cursor = use_signal(|| GhostCursorState::new());
    let mut ghost_click_rings: Signal<Vec<(f64, f64, u64)>> = use_signal(|| Vec::new());
    let mut _active_progress = use_signal(|| Option::<ProgressJourney>::None);
    let mut celebration = use_signal(|| Option::<Celebration>::None);
    let mut celebration_dismiss_scheduled = use_signal(|| false);
    let mut active_error = use_signal(|| Option::<FriendlyError>::None);
    let mut approval_countdown = use_signal(|| 0u32);

    // ── UI state ──
    let mut show_features = use_signal(|| false);
    let mut is_typing = use_signal(|| false);
    let mut phase_statuses = use_signal(|| Vec::<PhaseStatus>::new());
    let mut input_error = use_signal(|| Option::<String>::None);
    let mut show_command_palette = use_signal(|| false);
    let mut command_palette = use_signal(CommandPalette::new);
    let mut session_counter = use_signal(|| 1u32);
    let mut new_session_flash = use_signal(|| false);
    let mut active_session_id = use_signal(|| "session-1".to_string());
    let mut voice_listening = use_signal(|| false);
    let mut mic_stop_flag = use_signal(|| Arc::new(AtomicBool::new(false)));
    let mut show_search = use_signal(|| false);
    let mut search_query = use_signal(|| String::new());
    let mut show_receipts = use_signal(|| false);
    // Store messages per session: session_id -> Vec<(role, content, css)>
    let mut session_store = use_signal(|| HashMap::<String, Vec<(String, String, String)>>::new());

    // ── Undo/Redo state ──
    let mut can_undo = use_signal(|| false);
    let mut can_redo = use_signal(|| false);
    let mut last_undo_action = use_signal(|| Option::<String>::None);
    let undo_sig: Signal<Arc<parking_lot::Mutex<UndoStack>>> = use_signal(|| undo_stack.clone());

    // ── Workspace panels ──
    let mut plan_panel = use_signal(|| PlanPanel::default());
    let mut timeline_panel = use_signal(|| TimelinePanel::new());
    let mut evidence_panel = use_signal(|| EvidencePanel::new());

    // ── Global keyboard shortcuts via JS document listener ──
    let kb_approval_mgr = approval_manager.clone();
    use_hook(|| {
        spawn(async move {
            let mut eval = document::eval(r#"
                (async function() {
                    var queue = [];
                    document.addEventListener('keydown', function(e) {
                        if (e.metaKey || e.ctrlKey) {
                            var k = e.key.toLowerCase();
                            if (e.shiftKey && k === 'k') {
                                e.preventDefault();
                                queue.push('shift+k');
                            } else if (['k','b','n','f',',','z','1','2','3','4'].indexOf(k) !== -1) {
                                e.preventDefault();
                                queue.push(k);
                            }
                        }
                        if (e.key === 'Escape') {
                            queue.push('escape');
                        }
                    });
                    while (true) {
                        if (queue.length > 0) {
                            dioxus.send(queue.shift());
                        }
                        await new Promise(r => setTimeout(r, 16));
                    }
                })()
            "#);
            loop {
                match eval.recv::<String>().await {
                    Ok(key) => {
                        match key.as_str() {
                            "shift+k" => {
                                // Kill switch — emergency halt all activity
                                kb_approval_mgr.cancel_all();
                                is_typing.set(false);
                                phase.set("Idle".into());
                                icon_state.set("idle".into());
                                pending_approval.set(None);
                                pending_approval_id.set(None);
                                phase_statuses.set(vec![]);
                                active_error.set(Some(FriendlyError {
                                    message: "Kill Switch Activated".into(),
                                    explanation: "All operations halted. Press Escape to dismiss, or Cmd+N for a fresh session.".into(),
                                    options: vec![],
                                    icon_state: "error".into(),
                                    can_undo: false,
                                }));
                            }
                            "k" => {
                                let current = *show_command_palette.read();
                                show_command_palette.set(!current);
                                if !current { command_palette.write().reset(); }
                            }
                            "b" => {
                                let c = *show_sidebar.read();
                                show_sidebar.set(!c);
                            }
                            "," => {
                                show_settings.set(true);
                            }
                            "n" => {
                                // Save current session
                                let cur_id = active_session_id.read().clone();
                                let cur_msgs = messages.read().clone();
                                session_store.write().insert(cur_id, cur_msgs);
                                // Complete active tasks
                                let task_ids: Vec<String> = sidebar.read().today_items().iter()
                                    .filter(|item| item.active)
                                    .map(|item| item.id.clone())
                                    .collect();
                                for id in task_ids {
                                    sidebar.write().complete_task(&id);
                                }
                                // Create new DB conversation
                                let _conv_id = chat_db_sig.read().new_conversation();
                                // Create new session
                                let count = *session_counter.read() + 1;
                                session_counter.set(count);
                                let new_id = format!("session-{}", count);
                                sidebar.write().add_task(&new_id, &format!("Session {}", count));
                                active_session_id.set(new_id);
                                messages.write().clear();
                                phase.set("Idle".into());
                                icon_state.set("idle".into());
                                is_typing.set(false);
                                connected.set(false);
                                input.set(String::new());
                                plan_panel.write().steps.clear();
                                timeline_panel.write().clear();
                                evidence_panel.write().clear();
                                pending_approval.set(None);
                                pending_approval_id.set(None);
                                celebration.set(None);
                                active_error.set(None);
                                phase_statuses.set(vec![]);
                                challenge_input.set(String::new());
                                new_session_flash.set(true);
                            }
                            "f" => {
                                show_search.set(true);
                                search_query.set(String::new());
                            }
                            "1" => { current_mode.set("companion".into()); }
                            "2" => { current_mode.set("workspace".into()); show_sidebar.set(true); }
                            "3" => { current_mode.set("immersive".into()); }
                            "4" => { current_mode.set("invisible".into()); }
                            "z" => {
                                let stack = undo_sig.read();
                                let mut s = stack.lock();
                                if s.can_undo() {
                                    let _ = s.undo();
                                    can_undo.set(s.can_undo());
                                    can_redo.set(s.can_redo());
                                    last_undo_action.set(s.last_action_description().map(String::from));
                                }
                            }
                            "escape" => {
                                show_command_palette.set(false);
                                show_settings.set(false);
                                show_features.set(false);
                                show_receipts.set(false);
                                if *show_search.read() {
                                    show_search.set(false);
                                    search_query.set(String::new());
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    });

    // ── Greeting ──
    let user_name = onboarding.read().user_name.clone().unwrap_or_default();
    let greeting = if user_name.is_empty() { "Hi there!".to_string() } else { format!("Hi {}!", user_name) };

    // ── Send message handler + save profile (extracted for compilation memory) ──
    let (mut send_message, save_current_profile) = include!("app_send_handler.rs");

    // ── RSX (split into fragment files for compilation memory) ──
    include!("app_rsx.rs")
}
