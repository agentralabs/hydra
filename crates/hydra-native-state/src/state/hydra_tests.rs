//! Tests for HydraState, types, and serialization.

#[cfg(test)]
mod tests {
    use crate::state::hydra_types::*;
    use crate::state::hydra_state::HydraState;

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

    #[test]
    fn test_cognitive_phase_all_labels_non_empty() {
        for phase in CognitivePhase::ALL {
            assert!(!phase.label().is_empty());
        }
    }

    #[test]
    fn test_cognitive_phase_indices_sequential() {
        for (i, phase) in CognitivePhase::ALL.iter().enumerate() {
            assert_eq!(phase.index(), i);
        }
    }

    #[test]
    fn test_globe_state_all_css_classes_unique() {
        let states = [
            GlobeState::Idle,
            GlobeState::Listening,
            GlobeState::Processing,
            GlobeState::Speaking,
            GlobeState::Error,
            GlobeState::Approval,
        ];
        let classes: Vec<&str> = states.iter().map(|s| s.css_class()).collect();
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_event_log_respects_max_events() {
        let mut config = AppConfig::default();
        config.server_url = "http://test".into();
        let mut state = HydraState::new(config);
        // The max is 100; push 120 events via run starts
        for i in 0..120 {
            state.handle_run_started(&format!("run-{}", i), "test");
        }
        assert!(state.recent_events().len() <= 100);
    }

    #[test]
    fn test_step_completed_without_prior_start() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        // Complete a phase that was never started — should still add it
        state.handle_step_completed("run-1", CognitivePhase::Act, Some(50), Some(10));
        let run = state.current_run.as_ref().unwrap();
        assert_eq!(run.phases.len(), 1);
        assert_eq!(run.phases[0].state, PhaseState::Completed);
        assert_eq!(run.phases[0].phase, CognitivePhase::Act);
    }

    #[test]
    fn test_run_completed_without_response() {
        let mut state = HydraState::with_defaults();
        state.handle_run_started("run-1", "test");
        state.handle_run_completed("run-1", None, None);
        assert_eq!(state.globe_state, GlobeState::Idle);
        assert!(state.messages.is_empty()); // No response message added
    }

    #[test]
    fn test_set_connected() {
        let mut state = HydraState::with_defaults();
        assert!(!state.connected);
        state.set_connected(true);
        assert!(state.connected);
        state.set_connected(false);
        assert!(!state.connected);
    }

    #[test]
    fn test_theme_serialization() {
        let themes = [Theme::Dark, Theme::Light, Theme::System];
        for t in &themes {
            let json = serde_json::to_string(t).unwrap();
            let back: Theme = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.server_url, "http://localhost:3000");
        assert!((back.sound_volume - 0.7).abs() < f32::EPSILON);
        assert!(!back.auto_approve_low_risk);
    }

    #[test]
    fn test_message_role_serialization() {
        let roles = [MessageRole::User, MessageRole::Hydra];
        for r in &roles {
            let json = serde_json::to_string(r).unwrap();
            let back: MessageRole = serde_json::from_str(&json).unwrap();
            assert_eq!(*r, back);
        }
    }

    #[test]
    fn test_run_status_serialization() {
        let statuses = [RunStatus::Running, RunStatus::Completed, RunStatus::Failed, RunStatus::Cancelled];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: RunStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
