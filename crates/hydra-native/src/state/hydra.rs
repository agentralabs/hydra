//! Global application state — messages, phases, connection, runs.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Cognitive phase in the loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CognitivePhase {
    Perceive,
    Think,
    Decide,
    Act,
    Learn,
}

impl CognitivePhase {
    pub const ALL: &'static [CognitivePhase] = &[
        Self::Perceive,
        Self::Think,
        Self::Decide,
        Self::Act,
        Self::Learn,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Perceive => "Perceive",
            Self::Think => "Think",
            Self::Decide => "Decide",
            Self::Act => "Act",
            Self::Learn => "Learn",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::Perceive => 0,
            Self::Think => 1,
            Self::Decide => 2,
            Self::Act => 3,
            Self::Learn => 4,
        }
    }
}

/// Status of a phase in the current run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Phase tracking for the current run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseStatus {
    pub phase: CognitivePhase,
    pub state: PhaseState,
    pub tokens_used: Option<u64>,
    pub duration_ms: Option<u64>,
}

/// Voice globe animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobeState {
    Idle,
    Listening,
    Processing,
    Speaking,
    Error,
    Approval,
}

impl GlobeState {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Idle => "globe-idle",
            Self::Listening => "globe-listening",
            Self::Processing => "globe-processing",
            Self::Speaking => "globe-speaking",
            Self::Error => "globe-error",
            Self::Approval => "globe-approval",
        }
    }
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    pub run_id: Option<String>,
    pub tokens_used: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Hydra,
}

/// A running or completed run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub id: String,
    pub intent: String,
    pub status: RunStatus,
    pub phases: Vec<PhaseStatus>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub total_tokens: Option<u64>,
    pub response: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server_url: String,
    pub theme: Theme,
    pub voice_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:3000".into(),
            theme: Theme::Dark,
            voice_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}

/// The complete application state
pub struct HydraState {
    pub messages: Vec<ChatMessage>,
    pub current_run: Option<Run>,
    pub globe_state: GlobeState,
    pub connected: bool,
    pub config: AppConfig,
    pub error: Option<String>,
    /// Recent events for debugging
    event_log: VecDeque<String>,
    max_events: usize,
}

impl HydraState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            messages: Vec::new(),
            current_run: None,
            globe_state: GlobeState::Idle,
            connected: false,
            config,
            error: None,
            event_log: VecDeque::new(),
            max_events: 100,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AppConfig::default())
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.messages.push(ChatMessage {
            id: id.clone(),
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            run_id: None,
            tokens_used: None,
        });
        id
    }

    /// Add a hydra response message
    pub fn add_hydra_message(
        &mut self,
        content: &str,
        run_id: Option<&str>,
        tokens: Option<u64>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.messages.push(ChatMessage {
            id: id.clone(),
            role: MessageRole::Hydra,
            content: content.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            run_id: run_id.map(|s| s.to_string()),
            tokens_used: tokens,
        });
        id
    }

    /// Handle run_started SSE event
    pub fn handle_run_started(&mut self, run_id: &str, intent: &str) {
        self.current_run = Some(Run {
            id: run_id.to_string(),
            intent: intent.to_string(),
            status: RunStatus::Running,
            phases: Vec::new(),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            total_tokens: None,
            response: None,
        });
        self.globe_state = GlobeState::Processing;
        self.error = None;
        self.log_event("run_started");
    }

    /// Handle step_started SSE event
    pub fn handle_step_started(&mut self, run_id: &str, phase: CognitivePhase) {
        if let Some(run) = &mut self.current_run {
            if run.id == run_id {
                // Update or add phase
                if let Some(ps) = run.phases.iter_mut().find(|p| p.phase == phase) {
                    ps.state = PhaseState::Running;
                } else {
                    run.phases.push(PhaseStatus {
                        phase,
                        state: PhaseState::Running,
                        tokens_used: None,
                        duration_ms: None,
                    });
                }
            }
        }
        self.log_event(&format!("step_started:{:?}", phase));
    }

    /// Handle step_completed SSE event
    pub fn handle_step_completed(
        &mut self,
        run_id: &str,
        phase: CognitivePhase,
        tokens_used: Option<u64>,
        duration_ms: Option<u64>,
    ) {
        if let Some(run) = &mut self.current_run {
            if run.id == run_id {
                if let Some(ps) = run.phases.iter_mut().find(|p| p.phase == phase) {
                    ps.state = PhaseState::Completed;
                    ps.tokens_used = tokens_used;
                    ps.duration_ms = duration_ms;
                } else {
                    run.phases.push(PhaseStatus {
                        phase,
                        state: PhaseState::Completed,
                        tokens_used,
                        duration_ms,
                    });
                }
            }
        }
        self.log_event(&format!("step_completed:{:?}", phase));
    }

    /// Handle run_completed SSE event
    pub fn handle_run_completed(
        &mut self,
        run_id: &str,
        response: Option<&str>,
        tokens: Option<u64>,
    ) {
        if let Some(run) = &mut self.current_run {
            if run.id == run_id {
                run.status = RunStatus::Completed;
                run.completed_at = Some(chrono::Utc::now().to_rfc3339());
                run.total_tokens = tokens;
                run.response = response.map(|s| s.to_string());
            }
        }

        // Add response as hydra message
        if let Some(response) = response {
            self.add_hydra_message(response, Some(run_id), tokens);
        }

        self.globe_state = GlobeState::Idle;
        self.log_event("run_completed");
    }

    /// Handle run_error SSE event
    pub fn handle_run_error(&mut self, run_id: &str, error: &str) {
        if let Some(run) = &mut self.current_run {
            if run.id == run_id {
                run.status = RunStatus::Failed;
            }
        }
        self.error = Some(error.to_string());
        self.add_hydra_message(&format!("Error: {}", error), Some(run_id), None);
        self.globe_state = GlobeState::Error;
        self.log_event("run_error");
    }

    /// Handle approval_required SSE event
    pub fn handle_approval_required(&mut self) {
        self.globe_state = GlobeState::Approval;
        self.log_event("approval_required");
    }

    /// Set connection state
    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    /// Get active phase (the currently running phase, if any)
    pub fn active_phase(&self) -> Option<CognitivePhase> {
        self.current_run.as_ref().and_then(|run| {
            run.phases
                .iter()
                .find(|p| p.state == PhaseState::Running)
                .map(|p| p.phase)
        })
    }

    /// Get total tokens used across all messages
    pub fn total_tokens(&self) -> u64 {
        self.messages.iter().filter_map(|m| m.tokens_used).sum()
    }

    /// Log an internal event
    fn log_event(&mut self, event: &str) {
        self.event_log.push_back(event.to_string());
        while self.event_log.len() > self.max_events {
            self.event_log.pop_front();
        }
    }

    /// Get recent events (for debugging)
    pub fn recent_events(&self) -> Vec<&str> {
        self.event_log.iter().map(|s| s.as_str()).collect()
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_run = None;
        self.globe_state = GlobeState::Idle;
        self.error = None;
        self.event_log.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = HydraState::with_defaults();
        assert!(state.messages.is_empty());
        assert!(state.current_run.is_none());
        assert_eq!(state.globe_state, GlobeState::Idle);
        assert!(!state.connected);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_add_messages() {
        let mut state = HydraState::with_defaults();
        let id1 = state.add_user_message("Hello");
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, MessageRole::User);
        assert!(!id1.is_empty());

        let id2 = state.add_hydra_message("Hi there!", None, Some(50));
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[1].role, MessageRole::Hydra);
        assert_eq!(state.messages[1].tokens_used, Some(50));
        assert!(!id2.is_empty());
    }

    #[test]
    fn test_run_lifecycle() {
        let mut state = HydraState::with_defaults();

        // Start run
        state.handle_run_started("run-1", "test intent");
        assert!(state.current_run.is_some());
        assert_eq!(state.globe_state, GlobeState::Processing);

        // Phases progress
        state.handle_step_started("run-1", CognitivePhase::Perceive);
        assert_eq!(state.active_phase(), Some(CognitivePhase::Perceive));

        state.handle_step_completed("run-1", CognitivePhase::Perceive, Some(100), Some(50));
        assert_eq!(state.active_phase(), None); // No running phase

        state.handle_step_started("run-1", CognitivePhase::Think);
        assert_eq!(state.active_phase(), Some(CognitivePhase::Think));

        // Complete run
        state.handle_run_completed("run-1", Some("Result here"), Some(500));
        assert_eq!(state.globe_state, GlobeState::Idle);
        assert_eq!(
            state.current_run.as_ref().unwrap().status,
            RunStatus::Completed
        );
        // Response added as message
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "Result here");
    }

    #[test]
    fn test_run_error() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        state.handle_run_error("run-1", "Something broke");
        assert_eq!(state.globe_state, GlobeState::Error);
        assert_eq!(state.error.as_deref(), Some("Something broke"));
        assert_eq!(
            state.current_run.as_ref().unwrap().status,
            RunStatus::Failed
        );
        assert!(state.messages[0].content.contains("Error:"));
    }

    #[test]
    fn test_globe_states() {
        assert_eq!(GlobeState::Idle.css_class(), "globe-idle");
        assert_eq!(GlobeState::Processing.css_class(), "globe-processing");
        assert_eq!(GlobeState::Error.css_class(), "globe-error");
        assert_eq!(GlobeState::Approval.css_class(), "globe-approval");
    }

    #[test]
    fn test_phase_metadata() {
        assert_eq!(CognitivePhase::Perceive.label(), "Perceive");
        assert_eq!(CognitivePhase::Learn.index(), 4);
        assert_eq!(CognitivePhase::ALL.len(), 5);
    }

    #[test]
    fn test_approval_state() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        state.handle_approval_required();
        assert_eq!(state.globe_state, GlobeState::Approval);
    }

    #[test]
    fn test_event_log() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        state.handle_step_started("run-1", CognitivePhase::Perceive);
        let events = state.recent_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], "run_started");
    }

    #[test]
    fn test_total_tokens() {
        let mut state = HydraState::with_defaults();
        state.add_hydra_message("a", None, Some(100));
        state.add_hydra_message("b", None, Some(200));
        state.add_user_message("c"); // No tokens
        assert_eq!(state.total_tokens(), 300);
    }

    #[test]
    fn test_clear() {
        let mut state = HydraState::with_defaults();
        state.add_user_message("hello");
        state.handle_run_started("run-1", "test");
        state.clear();
        assert!(state.messages.is_empty());
        assert!(state.current_run.is_none());
        assert_eq!(state.globe_state, GlobeState::Idle);
    }

    #[test]
    fn test_wrong_run_id_ignored() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        state.handle_step_started("run-2", CognitivePhase::Perceive); // wrong ID
        assert_eq!(state.active_phase(), None); // Not updated
    }

    #[test]
    fn test_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.server_url, "http://localhost:3000");
        assert_eq!(config.theme, Theme::Dark);
        assert!(!config.voice_enabled);
    }
}
