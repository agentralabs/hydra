//! ComputerUseAgent — high-level task execution via screenshot→vision→action loop.
//! Uses VisionProvider to understand the screen and decide next actions.

use crate::action::BrowserAction;
use crate::constants::MAX_COMPUTER_USE_STEPS;
use crate::engine::BrowserEngine;
use crate::errors::BrowserError;
use crate::vision::VisionProvider;

use serde::{Deserialize, Serialize};

/// A single step in the computer use loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseStep {
    pub step_number: u32,
    pub action_taken: String,
    pub observation: String,
    pub is_complete: bool,
}

/// Result of a computer use task execution.
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub goal: String,
    pub completed: bool,
    pub steps: Vec<ComputerUseStep>,
    pub final_observation: String,
}

/// Executes multi-step browser tasks by iterating: screenshot → analyze → act.
pub struct ComputerUseAgent {
    max_steps: u32,
}

impl ComputerUseAgent {
    pub fn new() -> Self {
        Self {
            max_steps: MAX_COMPUTER_USE_STEPS,
        }
    }

    pub fn with_max_steps(max_steps: u32) -> Self {
        Self { max_steps }
    }

    /// Execute a high-level task (e.g., "Post 'Hello World' on Twitter").
    pub async fn execute_task(
        &self,
        engine: &mut BrowserEngine,
        goal: &str,
        vision: &dyn VisionProvider,
    ) -> Result<TaskResult, BrowserError> {
        let mut steps = Vec::new();
        let mut completed = false;

        eprintln!("hydra-browser: computer-use starting task: {goal}");

        for step_num in 1..=self.max_steps {
            let step = self.step(engine, goal, &steps, vision).await?;

            eprintln!(
                "hydra-browser: step {}/{}: {} (complete={})",
                step_num, self.max_steps, step.action_taken, step.is_complete
            );

            completed = step.is_complete;
            steps.push(step);

            if completed {
                break;
            }
        }

        if !completed && steps.len() as u32 >= self.max_steps {
            eprintln!(
                "hydra-browser: task exceeded {} steps without completion",
                self.max_steps
            );
        }

        let final_observation = steps
            .last()
            .map(|s| s.observation.clone())
            .unwrap_or_default();

        Ok(TaskResult {
            goal: goal.to_string(),
            completed,
            steps,
            final_observation,
        })
    }

    /// Execute a task with step-by-step updates sent to an mpsc channel.
    /// Each step emits an AgentUpdate-compatible message via the sender.
    pub async fn execute_task_with_updates(
        &self,
        engine: &mut BrowserEngine,
        goal: &str,
        vision: &dyn VisionProvider,
        updates: tokio::sync::mpsc::Sender<ComputerUseStep>,
    ) -> Result<TaskResult, BrowserError> {
        let mut steps = Vec::new();
        let mut completed = false;

        eprintln!("hydra-browser: computer-use (streaming) starting task: {goal}");

        for step_num in 1..=self.max_steps {
            let step = self.step(engine, goal, &steps, vision).await?;

            eprintln!(
                "hydra-browser: step {}/{}: {} (complete={})",
                step_num, self.max_steps, step.action_taken, step.is_complete
            );

            // Send step update (best-effort — if receiver is dropped, continue)
            let _ = updates.send(step.clone()).await;

            completed = step.is_complete;
            steps.push(step);

            if completed {
                break;
            }
        }

        let final_observation = steps
            .last()
            .map(|s| s.observation.clone())
            .unwrap_or_default();

        Ok(TaskResult {
            goal: goal.to_string(),
            completed,
            steps,
            final_observation,
        })
    }

    /// Execute a single step: screenshot → vision analysis → decide action → execute.
    async fn step(
        &self,
        engine: &mut BrowserEngine,
        goal: &str,
        previous_steps: &[ComputerUseStep],
        vision: &dyn VisionProvider,
    ) -> Result<ComputerUseStep, BrowserError> {
        let step_number = previous_steps.len() as u32 + 1;

        // 1. Take screenshot
        let screenshot = engine.screenshot().await?;

        // 2. Build context prompt
        let history = Self::format_history(previous_steps);
        let prompt = format!(
            "GOAL: {goal}\n\n\
             PREVIOUS STEPS:\n{history}\n\n\
             CURRENT STEP: {step_number}\n\n\
             Look at this screenshot. Decide the NEXT action to take toward the goal.\n\
             Respond in this exact JSON format:\n\
             {{\"action\": \"click|type|scroll|navigate|done\", \
             \"selector\": \"CSS selector if click/type\", \
             \"text\": \"text if type or URL if navigate\", \
             \"reasoning\": \"why this action\"}}\n\n\
             If the goal is complete, use action \"done\"."
        );

        // 3. Ask vision provider
        let response = vision.analyze_image(&screenshot, &prompt).await?;

        // 4. Parse the response
        let decision = Self::parse_vision_response(&response);

        // 5. Execute the decided action
        let (action_taken, observation) = match decision {
            VisionDecision::Click { selector } => {
                let result = engine
                    .execute(&BrowserAction::Click {
                        selector: selector.clone(),
                    })
                    .await;
                (format!("click: {selector}"), result.data)
            }
            VisionDecision::Type { selector, text } => {
                let result = engine
                    .execute(&BrowserAction::Type {
                        selector: selector.clone(),
                        text: text.clone(),
                    })
                    .await;
                (format!("type: '{text}' into {selector}"), result.data)
            }
            VisionDecision::Navigate { url } => {
                let result = engine
                    .execute(&BrowserAction::Navigate { url: url.clone() })
                    .await;
                (format!("navigate: {url}"), result.data)
            }
            VisionDecision::Scroll => {
                engine
                    .execute(&BrowserAction::Scroll {
                        direction: crate::action::ScrollDirection::Down,
                        amount: 300,
                    })
                    .await;
                ("scroll down".into(), "Scrolled".into())
            }
            VisionDecision::Done { reasoning } => {
                return Ok(ComputerUseStep {
                    step_number,
                    action_taken: "done".into(),
                    observation: reasoning,
                    is_complete: true,
                });
            }
            VisionDecision::Unknown { raw } => {
                ("unknown (vision unclear)".into(), raw)
            }
        };

        // Wait for page to settle after action
        engine.execute(&BrowserAction::Wait { ms: 1000 }).await;

        Ok(ComputerUseStep {
            step_number,
            action_taken,
            observation,
            is_complete: false,
        })
    }

    fn format_history(steps: &[ComputerUseStep]) -> String {
        if steps.is_empty() {
            return "(none — first step)".into();
        }
        steps
            .iter()
            .map(|s| format!("  Step {}: {} → {}", s.step_number, s.action_taken, s.observation))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn parse_vision_response(response: &str) -> VisionDecision {
        // Try to parse as JSON
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(response) {
            let action = val
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            match action {
                "click" => {
                    let selector = val
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .unwrap_or("button")
                        .to_string();
                    VisionDecision::Click { selector }
                }
                "type" => {
                    let selector = val
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .unwrap_or("input")
                        .to_string();
                    let text = val
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    VisionDecision::Type { selector, text }
                }
                "navigate" => {
                    let url = val
                        .get("text")
                        .or_else(|| val.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    VisionDecision::Navigate { url }
                }
                "scroll" => VisionDecision::Scroll,
                "done" => {
                    let reasoning = val
                        .get("reasoning")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Task appears complete")
                        .to_string();
                    VisionDecision::Done { reasoning }
                }
                _ => VisionDecision::Unknown {
                    raw: response.to_string(),
                },
            }
        } else {
            // Fallback: try to extract action from plain text
            VisionDecision::Unknown {
                raw: response.to_string(),
            }
        }
    }
}

impl Default for ComputerUseAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum VisionDecision {
    Click { selector: String },
    Type { selector: String, text: String },
    Navigate { url: String },
    Scroll,
    Done { reasoning: String },
    Unknown { raw: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_click_response() {
        let json = r##"{"action": "click", "selector": "#submit-btn", "reasoning": "Submit the form"}"##;
        let decision = ComputerUseAgent::parse_vision_response(json);
        assert!(matches!(decision, VisionDecision::Click { selector } if selector == "#submit-btn"), "Expected Click with #submit-btn");
    }

    #[test]
    fn parse_done_response() {
        let json = r#"{"action": "done", "reasoning": "The tweet was posted successfully"}"#;
        let decision = ComputerUseAgent::parse_vision_response(json);
        assert!(matches!(decision, VisionDecision::Done { .. }));
    }

    #[test]
    fn parse_type_response() {
        let json = r#"{"action": "type", "selector": "input#email", "text": "user@test.com"}"#;
        let decision = ComputerUseAgent::parse_vision_response(json);
        assert!(matches!(decision, VisionDecision::Type { selector, text } if selector == "input#email" && text == "user@test.com"));
    }

    #[test]
    fn invalid_json_returns_unknown() {
        let response = "I see a login page with username and password fields";
        let decision = ComputerUseAgent::parse_vision_response(response);
        assert!(matches!(decision, VisionDecision::Unknown { .. }));
    }

    #[test]
    fn format_empty_history() {
        let history = ComputerUseAgent::format_history(&[]);
        assert_eq!(history, "(none — first step)");
    }
}
