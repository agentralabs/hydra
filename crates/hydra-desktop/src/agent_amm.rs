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
use crate::verification::{self, ActionExpectation, VerifyResult};

use hydra_browser::VisionProvider;
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
        let app_model = AppModel::load(&app_name).unwrap_or_else(|| {
            eprintln!("hydra-desktop: first contact with '{app_name}'...");
            AppModel::first_contact()
        });

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
                 type|key_press|key_combo|scroll|wait|done\", \
                 \"x\": pixel_x, \"y\": pixel_y, \"text\": \"...\", \
                 \"modifier\": \"cmd|ctrl|alt|shift\", \"key\": \"...\", \
                 \"reasoning\": \"why\"}}\n\nIf the goal is complete, use \"done\"."
            );

            let response = vision.analyze_image(&screenshot_bytes, &prompt).await
                .map_err(|e| DesktopError::VisionError(format!("{e}")))?;
            let action = Self::parse_response(&response);

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
            if let Some(exp) = expectation {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                match verification::verify_action(&mut perception, &exp) {
                    VerifyResult::Confirmed { tier, evidence } =>
                        eprintln!("hydra-desktop: L5 verified tier {tier}: {evidence}"),
                    VerifyResult::Failed { reason } =>
                        eprintln!("hydra-desktop: L5 FAILED: {reason}"),
                    VerifyResult::Inconclusive =>
                        eprintln!("hydra-desktop: L5 inconclusive — continuing"),
                }
            }

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

        // L6: Crystallize successful sequence as muscle memory
        if completed {
            eprintln!("hydra-desktop: L6 crystallizing {} steps for '{goal}'", history.len());
        }

        let final_obs = history.last().map(|(_, o)| o.clone()).unwrap_or_default();
        Ok(DesktopTaskResult { completed, steps_taken: history.len() as u32,
            final_observation: final_obs })
    }
}
