//! HydraState — the complete application state and its methods.

use std::collections::VecDeque;

use super::hydra_types::*;

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
