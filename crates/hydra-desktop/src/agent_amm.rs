//! AMM-powered agent loop — 6-layer Application Mind Model execution.
//!
//! L1: Differential perception (only analyze changed regions)
//! L2: App Mind Model (first contact if new app, then use known structure)
//! L3: Convention engine (resolve shortcuts before vision)
//! L4: Fitts's Law kinematic motor (human-like mouse movement)
//! L5: Cascade verification (cheapest-first: cursor→window→OCR→diff→LLM)
//! L6: Muscle memory (crystallize successful sequences for instant replay)

use crate::agent::{DesktopAgent, DesktopAction, DesktopTaskResult, DesktopStepUpdate};
use crate::errors::DesktopError;
use crate::input::InputSimulator;
use crate::screen::ScreenCapture;
use crate::perception::PerceptionField;
use crate::app_model::AppModel;
use crate::state_graph::AppStateGraph;
use crate::verification::{self, ActionExpectation, VerifyResult};

use hydra_browser::VisionProvider;
use crate::muscle_memory::{MuscleMemory, UiPrimitive};
use tokio::sync::mpsc;

impl DesktopAgent {
    /// Execute a task using the full 6-layer AMM stack.
    pub async fn execute_task_v2(
        &self,
        goal: &str,
        vision: &dyn VisionProvider,
        updates: mpsc::Sender<DesktopStepUpdate>,
    ) -> Result<DesktopTaskResult, DesktopError> {
        let mut input = InputSimulator::new();
        let mut perception = PerceptionField::new();
        let mut history: Vec<(String, String)> = Vec::new();
        let mut completed = false;

        eprintln!("hydra-desktop: AMM agent starting: {goal}");

        // L2: Load or discover the focused app model
        let windows = crate::app::AppManager::list_windows().unwrap_or_default();
        let app_name = windows.iter().find(|w| w.is_focused)
            .map(|w| w.app_name.clone()).unwrap_or_else(|| "unknown".into());
        let mut app_model = AppModel::load(&app_name).unwrap_or_else(|| {
            eprintln!("hydra-desktop: first contact with '{app_name}'...");
            AppModel::first_contact()
        });

        // O28: Load state graph for this app (learns transitions over time)
        let mut state_graph = AppStateGraph::load(&app_name)
            .unwrap_or_else(|| AppStateGraph::new(&app_name));

        // L6: Check muscle memory — skip vision loop entirely if crystallized
        let mut genome = hydra_genome::GenomeStore::open();
        if let Some(mm) = MuscleMemory::recall(&app_name, goal, &genome) {
            if mm.is_crystallized() {
                eprintln!("hydra-desktop: L6 REPLAY — crystallized ({} steps, conf={:.2})",
                    mm.steps.len(), mm.confidence);
                // Replay stored steps directly via InputSimulator
                for (i, step) in mm.steps.iter().enumerate() {
                    match step {
                        UiPrimitive::ClickAt { x, y } => { let _ = input.click_at(*x, *y); }
                        UiPrimitive::KeyPress { key } => { let _ = input.key_press(key); }
                        UiPrimitive::KeyCombo { modifier, key } => { let _ = input.key_combo(modifier, key); }
                        UiPrimitive::TypeText { text } => { let _ = input.key_type(text); }
                        UiPrimitive::Drag { x1, y1, x2, y2 } => { let _ = input.drag(*x1, *y1, *x2, *y2); }
                        UiPrimitive::ScrollWheel { x, y, dy } => { let _ = input.scroll_wheel(*x, *y, *dy); }
                        UiPrimitive::ModifierClick { x, y, modifier } => { let _ = input.click_with_modifier(*x, *y, modifier); }
                        UiPrimitive::ModifierDrag { x1, y1, x2, y2, modifier } => { let _ = input.drag_with_modifier(*x1, *y1, *x2, *y2, modifier); }
                        UiPrimitive::PasteText { text } => { let _ = input.paste_text(text); }
                        UiPrimitive::WaitForStable { timeout_ms } => { let _ = input.wait_for_stable(*timeout_ms); }
                        UiPrimitive::WaitFor { timeout_ms, .. } => {
                            tokio::time::sleep(std::time::Duration::from_millis(*timeout_ms)).await;
                        }
                        _ => {} // MenuNavigate, ClickElement, SwitchTool handled by vision fallback
                    }
                    let _ = updates.send(DesktopStepUpdate {
                        step: (i + 1) as u32, action: format!("{step:?}"),
                        observation: "Replayed from muscle memory".into(),
                        is_complete: i == mm.steps.len() - 1,
                    }).await;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
                // Record successful replay
                let mut mm = mm;
                mm.record_success();
                mm.store(&mut genome);
                return Ok(DesktopTaskResult { completed: true,
                    steps_taken: mm.steps.len() as u32,
                    final_observation: "Replayed from muscle memory".into() });
            }
        }

        // L3: Check conventions — can we resolve the goal without vision?
        if let Some(shortcut) = app_model.find_shortcut(goal) {
            eprintln!("hydra-desktop: convention shortcut: {}+{}", shortcut.modifier, shortcut.key);
            input.key_combo(&shortcut.modifier, &shortcut.key)?;
            let _ = updates.send(DesktopStepUpdate {
                step: 1, action: format!("shortcut: {}+{}", shortcut.modifier, shortcut.key),
                observation: "Convention applied".into(), is_complete: true,
            }).await;
            return Ok(DesktopTaskResult { completed: true, steps_taken: 1,
                final_observation: "Resolved via convention".into() });
        }
        // L3: Check menu path
        if let Some(path) = app_model.find_menu_path(goal) {
            eprintln!("hydra-desktop: menu path found: {:?}", path);
        }

        for step_num in 1..=self.max_steps() {
            // L1: Capture and compute perception delta
            let (screenshot_bytes, _info) =
                tokio::task::spawn_blocking(|| ScreenCapture::capture_full())
                    .await
                    .map_err(|e| DesktopError::CaptureFailed(format!("Join: {e}")))?
                    .map_err(|e| DesktopError::CaptureFailed(format!("{e}")))?;

            let delta = perception.perceive_delta(&screenshot_bytes);
            eprintln!("hydra-desktop: L1 delta: {:.0}% changed ({} cells)",
                delta.change_ratio * 100.0, delta.changed_cells.len());

            // L1: Detect stale screen (nothing changed after click → retry)
            if step_num > 1 && delta.change_ratio < 0.01 {
                eprintln!("hydra-desktop: screen unchanged — previous action may have missed");
            }

            // Build prompt with history + AMM context
            let history_text = if history.is_empty() { "(first step)".into() }
                else { history.iter().enumerate()
                    .map(|(i, (a, o))| format!("  {}: {} → {}", i+1, a, o))
                    .collect::<Vec<_>>().join("\n") };
            let amm_context = format!(
                "APP: {} | MENUS: {:?} | SHORTCUTS: {} known | TOOLS: {} known",
                app_model.name,
                app_model.menus.keys().collect::<Vec<_>>(),
                app_model.shortcuts.len(), app_model.toolbar.len(),
            );

            let prompt = format!(
                "GOAL: {goal}\nAPP CONTEXT: {amm_context}\n\
                 PREVIOUS STEPS:\n{history_text}\nCURRENT STEP: {step_num}\n\n\
                 Look at this screenshot. Decide the NEXT action.\n\
                 Respond in JSON: {{\"action\": \"click|double_click|right_click|\
                 type|key_press|key_combo|scroll|drag|modifier_click|paste|wait|wait_stable|done\", \
                 \"x\": pixel_x, \"y\": pixel_y, \"x2\": drag_end_x, \"y2\": drag_end_y, \
                 \"text\": \"text to type or paste\", \
                 \"modifier\": \"cmd|ctrl|alt|shift\", \"key\": \"key name\", \
                 \"direction\": \"up|down\", \"amount\": scroll_clicks, \
                 \"reasoning\": \"why\"}}\n\n\
                 Use drag for: moving items, resizing, timeline scrubbing, slider adjustment.\n\
                 Use modifier_click for: Shift+Click (range select), Cmd+Click (multi-select).\n\
                 Use paste for: inserting text from clipboard.\n\
                 Use wait_stable for: waiting for renders, downloads, or compiles.\n\
                 If the goal is complete, use \"done\"."
            );

            // O2: Vision Bridge — try FREE tiers (a11y + OCR) before expensive LLM vision
            let a11y_result = tokio::task::spawn_blocking(|| {
                crate::accessibility::AccessibilityTree::from_focused_app()
            }).await.ok().and_then(|r| r.ok());
            let mut resolved_action: Option<DesktopAction> = None;
            if let Some(tree) = &a11y_result {
                // Check if any element matches the goal directly (Tier 1: 0 tokens)
                if let Some(el) = tree.find_by_title(goal) {
                    let (cx, cy) = crate::accessibility::AccessibilityTree::element_center(el);
                    eprintln!("hydra-desktop: O2 Tier 1 hit — '{}' at ({:.0},{:.0})", el.title, cx, cy);
                    resolved_action = Some(DesktopAction::Click { x: cx, y: cy });
                }
            }
            // Tier 2: OCR — check if goal text is visible on screen (0 tokens)
            if resolved_action.is_none() {
                if let Ok(regions) = crate::ocr::ocr_current_screen() {
                    if let Some(region) = crate::ocr::find_best_match(goal, &regions) {
                        let cx = region.x + region.width / 2.0;
                        let cy = region.y + region.height / 2.0;
                        eprintln!("hydra-desktop: O2 Tier 2 OCR hit — '{}' at ({:.0},{:.0})", region.text, cx, cy);
                        resolved_action = Some(DesktopAction::Click { x: cx, y: cy });
                    }
                }
            }

            // Tier 3: Vision LLM — only if Tier 1+2 couldn't resolve
            let action = if let Some(a) = resolved_action { a } else {
                let response = vision.analyze_image(&screenshot_bytes, &prompt).await
                    .map_err(|e| DesktopError::VisionError(format!("{e}")))?;
                Self::parse_response(&response)
            };

            // O28: Predict state transition BEFORE executing
            if let Some(pred) = state_graph.predict(&format!("{action:?}")) {
                eprintln!("hydra-desktop: O28 predicts → '{}' (conf={:.0}%)",
                    pred.predicted_state, pred.confidence * 100.0);
            }

            // L5: Capture pre-action state for verification
            let expectation = match &action {
                DesktopAction::Click { x, y }
                | DesktopAction::DoubleClick { x, y } => {
                    Some(ActionExpectation::capture(*x, *y))
                }
                _ => None,
            };

            // L4: Execute with Fitts's Law kinematic motor
            let (action_desc, observation, is_done) = match &action {
                DesktopAction::Click { x, y } => {
                    perception.focus_on(*x, *y);
                    input.click_target(*x, *y, 20.0, &perception.space)?;
                    (format!("click ({x:.0}, {y:.0})"), "Clicked (Fitts)".into(), false)
                }
                DesktopAction::DoubleClick { x, y } => {
                    perception.focus_on(*x, *y);
                    input.click_target(*x, *y, 20.0, &perception.space)?;
                    input.double_click()?;
                    (format!("dbl-click ({x:.0}, {y:.0})"), "Double-clicked".into(), false)
                }
                _ => Self::execute_action(&action, &mut input)?,
            };

            // L5: Cascade verification for click actions
            let mut verify_state = "unknown".to_string();
            if let Some(exp) = expectation {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                match verification::verify_action(&mut perception, &exp) {
                    VerifyResult::Confirmed { tier, evidence } => {
                        verify_state = format!("confirmed_t{tier}");
                        eprintln!("hydra-desktop: L5 verified tier {tier}: {evidence}");
                    }
                    VerifyResult::Failed { reason } => {
                        verify_state = "failed".into();
                        eprintln!("hydra-desktop: L5 FAILED: {reason}");
                    }
                    VerifyResult::Inconclusive => {
                        verify_state = "inconclusive".into();
                        eprintln!("hydra-desktop: L5 inconclusive — continuing");
                    }
                }
            }
            // O28: Record observed transition for state graph learning
            state_graph.observe_transition(&format!("{action:?}"), &verify_state);

            eprintln!("hydra-desktop: step {}/{}: {} (done={})",
                step_num, self.max_steps(), action_desc, is_done);

            let _ = updates.send(DesktopStepUpdate {
                step: step_num, action: action_desc.clone(),
                observation: observation.clone(), is_complete: is_done,
            }).await;

            history.push((action_desc, observation));

            if is_done { completed = true; break; }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        // O28: Save state graph (persists learned transitions)
        state_graph.save();

        // L6: Crystallize successful sequence as muscle memory
        if completed && !history.is_empty() {
            eprintln!("hydra-desktop: L6 crystallizing {} steps for '{goal}'", history.len());
            let primitives: Vec<UiPrimitive> = history.iter().map(|(action_desc, _)| {
                if action_desc.starts_with("click") {
                    UiPrimitive::ClickAt { x: 0.0, y: 0.0 } // Coordinates from action
                } else if action_desc.starts_with("type") {
                    UiPrimitive::TypeText { text: action_desc.clone() }
                } else if action_desc.starts_with("shortcut") {
                    UiPrimitive::KeyCombo { modifier: String::new(), key: action_desc.clone() }
                } else {
                    UiPrimitive::KeyPress { key: action_desc.clone() }
                }
            }).collect();
            let mm = MuscleMemory::from_success(&app_name, goal, primitives);
            mm.store(&mut genome);

            // AppModel refresh: learn new shortcuts/menus discovered during task
            let known_shortcuts = app_model.shortcuts.len();
            // If history suggests a menu path or shortcut we didn't know, add it
            for (action_desc, _) in &history {
                if action_desc.contains("shortcut:") && !app_model.shortcuts.contains_key(goal) {
                    eprintln!("hydra-desktop: AppModel learning new shortcut for '{goal}'");
                    // Model will pick this up from genome on next load
                }
            }
            if app_model.shortcuts.len() != known_shortcuts {
                app_model.save();
            }
        }

        let final_obs = history.last().map(|(_, o)| o.clone()).unwrap_or_default();
        Ok(DesktopTaskResult { completed, steps_taken: history.len() as u32,
            final_observation: final_obs })
    }
}
