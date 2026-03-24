//! DesktopAgent — vision-driven desktop automation via screenshot→analyze→act loop.
//!
//! Uses ScreenCapture + InputSimulator + VisionProvider to control any application.
//! Coordinate-based (pixel x,y) instead of CSS selectors.

use crate::errors::DesktopError;
use crate::input::InputSimulator;
use crate::screen::ScreenCapture;

use hydra_browser::VisionProvider;
use tokio::sync::mpsc;

/// Result of a desktop agent task execution.
#[derive(Debug, Clone)]
pub struct DesktopTaskResult {
    pub completed: bool,
    pub steps_taken: u32,
    pub final_observation: String,
}

/// A single decision parsed from vision response.
#[derive(Debug)]
enum DesktopAction {
    Click { x: f64, y: f64 },
    DoubleClick { x: f64, y: f64 },
    RightClick { x: f64, y: f64 },
    Type { text: String },
    KeyPress { key: String },
    KeyCombo { modifier: String, key: String },
    Scroll { direction: String, amount: u32 },
    Wait { ms: u64 },
    Done { reasoning: String },
    Unknown { raw: String },
}

/// Agent update sent via channel during execution.
#[derive(Debug, Clone)]
pub struct DesktopStepUpdate {
    pub step: u32,
    pub action: String,
    pub observation: String,
    pub is_complete: bool,
}

/// Executes multi-step desktop tasks via screenshot→vision→act loop.
pub struct DesktopAgent {
    max_steps: u32,
}

impl DesktopAgent {
    pub fn new() -> Self {
        Self { max_steps: 25 }
    }

    pub fn with_max_steps(max_steps: u32) -> Self {
        Self { max_steps }
    }

    /// Execute a desktop task with step updates sent to a channel.
    /// The channel receives AgentUpdate-compatible messages from agent_task.rs.
    pub async fn execute_task(
        &self,
        goal: &str,
        vision: &dyn VisionProvider,
        updates: mpsc::Sender<crate::agent::DesktopStepUpdate>,
    ) -> Result<DesktopTaskResult, DesktopError> {
        let mut input = InputSimulator::new();
        let mut history: Vec<(String, String)> = Vec::new();
        let mut completed = false;

        eprintln!("hydra-desktop: agent starting task: {goal}");

        for step_num in 1..=self.max_steps {
            // 1. Capture screenshot (blocking — wrap in spawn_blocking)
            let (screenshot_bytes, _info) =
                tokio::task::spawn_blocking(|| ScreenCapture::capture_full())
                    .await
                    .map_err(|e| DesktopError::CaptureFailed(format!("Join error: {e}")))?
                    .map_err(|e| DesktopError::CaptureFailed(format!("{e}")))?;

            // 2. Build prompt with history
            let history_text = if history.is_empty() {
                "(none — first step)".to_string()
            } else {
                history
                    .iter()
                    .enumerate()
                    .map(|(i, (a, o))| format!("  Step {}: {} → {}", i + 1, a, o))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let prompt = format!(
                "GOAL: {goal}\n\n\
                 PREVIOUS STEPS:\n{history_text}\n\n\
                 CURRENT STEP: {step_num}\n\n\
                 Look at this screenshot of a desktop application. \
                 Decide the NEXT action to take toward the goal.\n\
                 Respond in this exact JSON format:\n\
                 {{\"action\": \"click|double_click|right_click|type|key_press|key_combo|scroll|wait|done\", \
                 \"x\": pixel_x, \"y\": pixel_y, \
                 \"text\": \"text if type/key_press\", \
                 \"modifier\": \"cmd|ctrl|alt|shift\", \"key\": \"key name\", \
                 \"reasoning\": \"why this action\"}}\n\n\
                 If the goal is complete, use action \"done\"."
            );

            // 3. Ask vision provider
            let response = vision
                .analyze_image(&screenshot_bytes, &prompt)
                .await
                .map_err(|e| DesktopError::VisionError(format!("{e}")))?;

            // 4. Parse and execute
            let action = Self::parse_response(&response);
            let (action_desc, observation, is_done) =
                Self::execute_action(&action, &mut input)?;

            eprintln!(
                "hydra-desktop: step {}/{}: {} (done={})",
                step_num, self.max_steps, action_desc, is_done
            );

            // 5. Send update
            let _ = updates
                .send(DesktopStepUpdate {
                    step: step_num,
                    action: action_desc.clone(),
                    observation: observation.clone(),
                    is_complete: is_done,
                })
                .await;

            history.push((action_desc, observation.clone()));

            if is_done {
                completed = true;
                break;
            }

            // 6. Wait for UI to settle
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let final_observation = history
            .last()
            .map(|(_, o)| o.clone())
            .unwrap_or_default();

        Ok(DesktopTaskResult {
            completed,
            steps_taken: history.len() as u32,
            final_observation,
        })
    }

    fn parse_response(response: &str) -> DesktopAction {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(response) {
            let action = val
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let x = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let text = val
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let modifier = val
                .get("modifier")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let key = val
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let reasoning = val
                .get("reasoning")
                .and_then(|v| v.as_str())
                .unwrap_or("Task appears complete")
                .to_string();

            match action {
                "click" => DesktopAction::Click { x, y },
                "double_click" => DesktopAction::DoubleClick { x, y },
                "right_click" => DesktopAction::RightClick { x, y },
                "type" => DesktopAction::Type { text },
                "key_press" => DesktopAction::KeyPress { key: if key.is_empty() { text } else { key } },
                "key_combo" => DesktopAction::KeyCombo { modifier, key },
                "scroll" => {
                    let dir = val.get("direction").and_then(|v| v.as_str()).unwrap_or("down").to_string();
                    let amt = val.get("amount").and_then(|v| v.as_u64()).unwrap_or(300) as u32;
                    DesktopAction::Scroll { direction: dir, amount: amt }
                }
                "wait" => {
                    let ms = val.get("ms").and_then(|v| v.as_u64()).unwrap_or(1000);
                    DesktopAction::Wait { ms }
                }
                "done" => DesktopAction::Done { reasoning },
                _ => DesktopAction::Unknown { raw: response.to_string() },
            }
        } else {
            DesktopAction::Unknown {
                raw: response.to_string(),
            }
        }
    }

    fn execute_action(
        action: &DesktopAction,
        input: &mut InputSimulator,
    ) -> Result<(String, String, bool), DesktopError> {
        match action {
            DesktopAction::Click { x, y } => {
                input.click_at(*x, *y)?;
                Ok((format!("click ({x}, {y})"), "Clicked".into(), false))
            }
            DesktopAction::DoubleClick { x, y } => {
                input.click_at(*x, *y)?;
                input.double_click()?;
                Ok((format!("double-click ({x}, {y})"), "Double-clicked".into(), false))
            }
            DesktopAction::RightClick { x, y } => {
                input.click_at(*x, *y)?;
                input.right_click()?;
                Ok((format!("right-click ({x}, {y})"), "Right-clicked".into(), false))
            }
            DesktopAction::Type { text } => {
                input.key_type(text)?;
                Ok((format!("type: '{text}'"), "Typed".into(), false))
            }
            DesktopAction::KeyPress { key } => {
                input.key_press(key)?;
                Ok((format!("key: {key}"), "Pressed".into(), false))
            }
            DesktopAction::KeyCombo { modifier, key } => {
                input.key_combo(modifier, key)?;
                Ok((format!("combo: {modifier}+{key}"), "Pressed".into(), false))
            }
            DesktopAction::Scroll { direction, amount } => {
                // Use key-based scrolling as a simple approach
                let key = if direction == "up" { "Up" } else { "Down" };
                for _ in 0..(*amount / 100).max(1) {
                    input.key_press(key)?;
                }
                Ok((format!("scroll {direction} {amount}px"), "Scrolled".into(), false))
            }
            DesktopAction::Wait { ms } => {
                std::thread::sleep(std::time::Duration::from_millis(*ms));
                Ok((format!("wait {ms}ms"), "Waited".into(), false))
            }
            DesktopAction::Done { reasoning } => {
                Ok(("done".into(), reasoning.clone(), true))
            }
            DesktopAction::Unknown { raw } => {
                Ok(("unknown (vision unclear)".into(), raw.clone(), false))
            }
        }
    }
}

impl Default for DesktopAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_click_response() {
        let json = r#"{"action": "click", "x": 450, "y": 320, "reasoning": "Click the button"}"#;
        let action = DesktopAgent::parse_response(json);
        assert!(matches!(action, DesktopAction::Click { x, y } if (x - 450.0).abs() < 0.1 && (y - 320.0).abs() < 0.1));
    }

    #[test]
    fn parse_type_response() {
        let json = r#"{"action": "type", "text": "hello world"}"#;
        let action = DesktopAgent::parse_response(json);
        assert!(matches!(action, DesktopAction::Type { text } if text == "hello world"));
    }

    #[test]
    fn parse_key_combo_response() {
        let json = r#"{"action": "key_combo", "modifier": "cmd", "key": "s"}"#;
        let action = DesktopAgent::parse_response(json);
        assert!(matches!(action, DesktopAction::KeyCombo { modifier, key } if modifier == "cmd" && key == "s"));
    }

    #[test]
    fn parse_done_response() {
        let json = r#"{"action": "done", "reasoning": "Task complete"}"#;
        let action = DesktopAgent::parse_response(json);
        assert!(matches!(action, DesktopAction::Done { .. }));
    }

    #[test]
    fn parse_invalid_json() {
        let response = "This is not JSON";
        let action = DesktopAgent::parse_response(response);
        assert!(matches!(action, DesktopAction::Unknown { .. }));
    }
}
