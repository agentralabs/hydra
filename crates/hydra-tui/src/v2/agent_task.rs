//! Agent task — async computer-use agent spawned from conversation.
//! Runs ComputerUseAgent (browser) or DesktopAgent with step-by-step streaming.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::stream::ConversationStream;
use crate::stream_types::StreamItem;

/// Progress updates from a running computer-use agent.
#[derive(Debug, Clone)]
pub enum AgentUpdate {
    StepStarted { step: u32, description: String },
    ActionExecuted { step: u32, action: String, observation: String, is_complete: bool },
    Error(String),
    Done { steps_taken: u32, completed: bool, summary: String },
}

/// Create the vision provider from environment. Returns None if ANTHROPIC_API_KEY is not set.
pub fn create_vision_provider() -> Option<Arc<dyn hydra_browser::VisionProvider>> {
    let provider = hydra_kernel::vision_bridge::LlmVisionProvider::new()?;
    Some(Arc::new(provider))
}

/// Spawn a browser computer-use agent with vision. Returns a receiver for progress updates.
pub fn spawn_browser_agent(
    rt: &tokio::runtime::Runtime,
    goal: String,
    vision: Option<Arc<dyn hydra_browser::VisionProvider>>,
) -> mpsc::Receiver<AgentUpdate> {
    let (tx, rx) = mpsc::channel(64);
    rt.spawn(async move { run_browser_agent(goal, vision, tx).await });
    rx
}

/// Spawn a desktop computer-use agent with vision. Returns a receiver for progress updates.
pub fn spawn_desktop_agent(
    rt: &tokio::runtime::Runtime,
    goal: String,
    vision: Option<Arc<dyn hydra_browser::VisionProvider>>,
) -> mpsc::Receiver<AgentUpdate> {
    let (tx, rx) = mpsc::channel(64);
    rt.spawn(async move { run_desktop_agent(goal, vision, tx).await });
    rx
}

async fn run_browser_agent(
    goal: String,
    vision: Option<Arc<dyn hydra_browser::VisionProvider>>,
    tx: mpsc::Sender<AgentUpdate>,
) {
    let vision = match vision {
        Some(v) => v,
        None => {
            let _ = tx.send(AgentUpdate::Error(
                "No vision provider. Set ANTHROPIC_API_KEY for computer use.".into(),
            )).await;
            return;
        }
    };

    let _ = tx.send(AgentUpdate::StepStarted {
        step: 0, description: format!("Launching browser for: {goal}"),
    }).await;

    let mut engine = hydra_browser::BrowserEngine::new();
    if let Err(e) = engine.launch().await {
        let _ = tx.send(AgentUpdate::Error(format!("Chrome launch failed: {e}"))).await;
        return;
    }

    // Navigate to URL if found in goal, otherwise search the web for the goal
    let nav_target = if let Some(url) = extract_url(&goal) {
        if url.starts_with("http://") || url.starts_with("https://") {
            url
        } else {
            format!("https://{url}")
        }
    } else {
        // No explicit URL — open a search engine so the agent has a page to work with
        let query = goal.replace(' ', "+");
        format!("https://www.google.com/search?q={query}")
    };
    let _ = tx.send(AgentUpdate::StepStarted {
        step: 1, description: format!("Navigating to {nav_target}"),
    }).await;
    if let Err(e) = engine.navigate(&nav_target).await {
        let _ = tx.send(AgentUpdate::Error(format!("Navigation failed: {e}"))).await;
        engine.close().await;
        return;
    }

    // Try semantic affordance navigation first (50ms, 0 tokens)
    let nav_url = extract_url(&goal).unwrap_or_default();
    let sem_tx = tx.clone();
    let sem_update = move |step: u32, desc: &str, obs: &str, done: bool| {
        let _ = sem_tx.try_send(AgentUpdate::ActionExecuted {
            step, action: desc.to_string(), observation: obs.to_string(), is_complete: done,
        });
    };
    match hydra_semantic_nav::try_semantic_nav_with_url(&mut engine, &goal, &nav_url, &sem_update).await {
        hydra_semantic_nav::NavResult::Success => {
            let _ = tx.send(AgentUpdate::Done {
                steps_taken: 0, completed: true,
                summary: "Completed via semantic navigation (0 vision tokens)".into(),
            }).await;
            engine.close().await;
            return;
        }
        hydra_semantic_nav::NavResult::Unparseable(reason) => {
            eprintln!("hydra-semantic-nav: falling back to vision — {reason}");
        }
    }

    // Fallback: vision-based computer-use agent
    let agent = hydra_browser::ComputerUseAgent::new();
    let budget = hydra_browser::VisionBudget::new(100);
    let budgeted = hydra_browser::BudgetedVision::new(VisionArc(vision), budget);
    let (step_tx, mut step_rx) = mpsc::channel::<hydra_browser::ComputerUseStep>(32);

    let bridge_tx = tx.clone();
    let bridge = tokio::spawn(async move {
        while let Some(step) = step_rx.recv().await {
            let _ = bridge_tx.send(AgentUpdate::ActionExecuted {
                step: step.step_number, action: step.action_taken,
                observation: step.observation, is_complete: step.is_complete,
            }).await;
        }
    });

    match agent.execute_task_with_updates(&mut engine, &goal, &budgeted, step_tx).await {
        Ok(result) => {
            let _ = bridge.await;
            let _ = tx.send(AgentUpdate::Done {
                steps_taken: result.steps.len() as u32,
                completed: result.completed, summary: result.final_observation,
            }).await;
        }
        Err(e) => {
            let _ = tx.send(AgentUpdate::Error(format!("Agent error: {e}"))).await;
        }
    }
    engine.close().await;
}

async fn run_desktop_agent(
    goal: String,
    vision: Option<Arc<dyn hydra_browser::VisionProvider>>,
    tx: mpsc::Sender<AgentUpdate>,
) {
    let vision = match vision {
        Some(v) => v,
        None => {
            let _ = tx.send(AgentUpdate::Error(
                "No vision provider. Set ANTHROPIC_API_KEY for desktop agent.".into(),
            )).await;
            return;
        }
    };

    let _ = tx.send(AgentUpdate::StepStarted {
        step: 0, description: format!("Starting desktop agent for: {goal}"),
    }).await;

    let agent = hydra_desktop::agent::DesktopAgent::new();
    let vision_ref = VisionArc(vision);
    let (step_tx, mut step_rx) = mpsc::channel::<hydra_desktop::agent::DesktopStepUpdate>(32);

    let bridge_tx = tx.clone();
    let bridge = tokio::spawn(async move {
        while let Some(step) = step_rx.recv().await {
            let _ = bridge_tx.send(AgentUpdate::ActionExecuted {
                step: step.step, action: step.action,
                observation: step.observation, is_complete: step.is_complete,
            }).await;
        }
    });

    match agent.execute_task_v2(&goal, &vision_ref, step_tx).await {
        Ok(result) => {
            let _ = bridge.await;
            let _ = tx.send(AgentUpdate::Done {
                steps_taken: result.steps_taken, completed: result.completed,
                summary: result.final_observation,
            }).await;
        }
        Err(e) => {
            let _ = tx.send(AgentUpdate::Error(format!("Desktop agent error: {e}"))).await;
        }
    }
}

/// Drain agent updates into the conversation stream.
/// Returns true when the agent task is done and the channel can be dropped.
pub fn drain_agent(rx: &mut mpsc::Receiver<AgentUpdate>, stream: &mut ConversationStream) -> bool {
    while let Ok(update) = rx.try_recv() {
        match update {
            AgentUpdate::StepStarted { step, description } => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Step {step}: {description}"),
                    timestamp: chrono::Utc::now(),
                });
            }
            AgentUpdate::ActionExecuted { step, action, observation, is_complete } => {
                stream.push(StreamItem::AgentStep {
                    id: uuid::Uuid::new_v4(), step_number: step,
                    action, observation, is_complete, timestamp: chrono::Utc::now(),
                });
            }
            AgentUpdate::Error(e) => {
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Agent error: {e}"),
                    timestamp: chrono::Utc::now(),
                });
                stream.scroll_to_bottom();
                return true;
            }
            AgentUpdate::Done { steps_taken, completed, summary } => {
                let status = if completed { "completed" } else { "stopped" };
                stream.push(StreamItem::SystemNotification {
                    id: uuid::Uuid::new_v4(),
                    content: format!("Agent {status} after {steps_taken} steps"),
                    timestamp: chrono::Utc::now(),
                });
                if !summary.is_empty() {
                    stream.push(StreamItem::AssistantText {
                        id: uuid::Uuid::new_v4(), text: summary, timestamp: chrono::Utc::now(),
                    });
                }
                stream.scroll_to_bottom();
                return true;
            }
        }
        stream.scroll_to_bottom();
    }
    false
}

fn extract_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            return Some(word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c)).to_string());
        }
        if word.contains('.') && !word.starts_with('.') && word.len() > 3 {
            let clean = word.trim_end_matches(|c: char| ".,;:!?)\"'".contains(c));
            if clean.contains('.') { return Some(clean.to_string()); }
        }
    }
    None
}

/// Bridge for semantic nav updates → AgentUpdate channel.
struct SemNavUpdate(mpsc::Sender<AgentUpdate>);

impl SemNavUpdate {
    fn send_update(&self, step: u32, desc: &str, obs: &str, done: bool) {
        let _ = self.0.try_send(AgentUpdate::ActionExecuted {
            step, action: desc.to_string(), observation: obs.to_string(), is_complete: done,
        });
    }
}

/// Wrapper to implement VisionProvider for Arc<dyn VisionProvider>.
/// Needed because async_trait doesn't auto-impl for Arc.
struct VisionArc(Arc<dyn hydra_browser::VisionProvider>);

#[async_trait::async_trait]
impl hydra_browser::VisionProvider for VisionArc {
    async fn analyze_image(
        &self, image_bytes: &[u8], prompt: &str,
    ) -> Result<String, hydra_browser::BrowserError> {
        self.0.analyze_image(image_bytes, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_url_from_goal() {
        assert_eq!(extract_url("post hello on https://twitter.com"), Some("https://twitter.com".into()));
        assert_eq!(extract_url("open linkedin.com"), Some("linkedin.com".into()));
        assert_eq!(extract_url("do something"), None);
    }

    #[test]
    fn drain_returns_true_on_done() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let (tx, mut rx) = mpsc::channel(256);
        rt.block_on(async {
            tx.send(AgentUpdate::Done { steps_taken: 3, completed: true, summary: "Task done".into() }).await.unwrap();
        });
        let mut stream = crate::stream::ConversationStream::new();
        assert!(drain_agent(&mut rx, &mut stream));
    }

    #[test]
    fn drain_returns_true_on_error() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let (tx, mut rx) = mpsc::channel(256);
        rt.block_on(async {
            tx.send(AgentUpdate::Error("test error".into())).await.unwrap();
        });
        let mut stream = crate::stream::ConversationStream::new();
        assert!(drain_agent(&mut rx, &mut stream));
    }

    #[test]
    fn create_vision_provider_depends_on_env() {
        // Without guaranteed ANTHROPIC_API_KEY, just verify it doesn't panic
        let _provider = create_vision_provider();
    }
}
