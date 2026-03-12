//! Tests for ghost cursor components.

#[cfg(test)]
mod tests {
    use crate::components::ghost_cursor::*;

    #[test]
    fn test_new_cursor_state() {
        let state = GhostCursorState::new();
        assert!(!state.visible);
        assert_eq!(state.mode, CursorMode::Visible);
        assert_eq!(state.x, 0.0);
        assert_eq!(state.y, 0.0);
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_move_to_updates_position() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(100.0, 200.0, Some("Opening Chrome".into()));
        assert_eq!(state.x, 100.0);
        assert_eq!(state.y, 200.0);
        assert_eq!(state.action_label, Some("Opening Chrome".into()));
        assert_eq!(state.visual_state, CursorVisualState::Moving);
    }

    #[test]
    fn test_trail_grows_on_move() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(10.0, 20.0, None);
        state.move_to(30.0, 40.0, None);
        state.move_to(50.0, 60.0, None);
        assert_eq!(state.trail.len(), 3); // origin + 2 previous positions
    }

    #[test]
    fn test_trail_capped_at_max() {
        let mut state = GhostCursorState::new();
        state.show();
        state.max_trail = 5;
        for i in 0..20 {
            state.move_to(i as f64, i as f64, None);
        }
        assert!(state.trail.len() <= 5);
    }

    #[test]
    fn test_invisible_mode_no_trail() {
        let mut state = GhostCursorState::new();
        state.set_mode(CursorMode::Invisible);
        state.move_to(100.0, 200.0, None);
        assert!(state.trail.is_empty());
        assert!(!state.visible);
    }

    #[test]
    fn test_click_visual_state() {
        let mut state = GhostCursorState::new();
        state.click();
        assert_eq!(state.visual_state, CursorVisualState::Clicking);
    }

    #[test]
    fn test_typing_visual_state() {
        let mut state = GhostCursorState::new();
        state.start_typing();
        assert_eq!(state.visual_state, CursorVisualState::Typing);
    }

    #[test]
    fn test_pause_and_resume() {
        let mut state = GhostCursorState::new();
        state.pause();
        assert!(state.paused);
        assert_eq!(state.visual_state, CursorVisualState::Thinking);
        state.resume();
        assert!(!state.paused);
        assert_eq!(state.visual_state, CursorVisualState::Idle);
    }

    #[test]
    fn test_css_class_generation() {
        let mut state = GhostCursorState::new();
        assert!(state.css_class().contains("ghost-cursor-hidden"));

        state.show();
        state.click();
        let cls = state.css_class();
        assert!(cls.contains("ghost-cursor-clicking"));
        assert!(!cls.contains("ghost-cursor-hidden"));
    }

    #[test]
    fn test_pupil_offset() {
        let mut state = GhostCursorState::new();
        state.move_to(100.0, 0.0, None); // Move right
        let (dx, dy) = state.pupil_offset();
        assert!(dx > 0.0); // Looking right
        assert!(dy.abs() < 0.5); // Not much vertical
    }

    #[test]
    fn test_interpolate_arc() {
        let points = interpolate_arc(0.0, 0.0, 100.0, 100.0, 10);
        assert_eq!(points.len(), 11);
        assert_eq!(points[0], (0.0, 0.0));
        // End point should be close to target (arc offset returns to 0 at end)
        let last = points.last().unwrap();
        assert!((last.0 - 100.0).abs() < 0.01);
        assert!((last.1 - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_movement_duration() {
        let d = movement_duration_ms(500.0, CursorMode::Visible);
        assert!(d >= 100 && d <= 800);

        let fast = movement_duration_ms(500.0, CursorMode::Fast);
        assert!(fast < d);

        let invisible = movement_duration_ms(500.0, CursorMode::Invisible);
        assert_eq!(invisible, 0);
    }

    #[test]
    fn test_cursor_distance() {
        let d = cursor_distance(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_cursor_session_recording() {
        let mut session = CursorSession::new("task-1");
        assert_eq!(session.event_count(), 0);

        session.record(CursorAction::MoveTo { x: 100.0, y: 200.0, label: None }, 100.0, 200.0);
        session.record(CursorAction::Click { button: MouseButton::Left }, 100.0, 200.0);
        assert_eq!(session.event_count(), 2);

        session.finish();
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_cursor_svg_generation() {
        let svg = cursor_svg(0.5, -0.3);
        assert!(svg.contains("svg"));
        assert!(svg.contains("6495ED"));
        assert!(svg.contains("13.5")); // left pupil shifted
    }

    #[test]
    fn test_tick_trail_removes_old() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(10.0, 10.0, None);
        state.move_to(20.0, 20.0, None);
        assert_eq!(state.trail.len(), 2);

        // Age past expiry
        state.tick_trail(1100);
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_mode_switching() {
        let mut state = GhostCursorState::new();
        state.show();
        assert!(state.trail_enabled);

        state.set_mode(CursorMode::Fast);
        assert!(!state.trail_enabled);

        state.set_mode(CursorMode::Visible);
        assert!(state.trail_enabled);

        state.set_mode(CursorMode::Replay);
        assert_eq!(state.replay_progress, 0.0);
    }

    #[test]
    fn test_cursor_action_serialization() {
        let action = CursorAction::MoveTo { x: 100.0, y: 200.0, label: Some("test".into()) };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("move_to"));
        let restored: CursorAction = serde_json::from_str(&json).unwrap();
        match restored {
            CursorAction::MoveTo { x, y, label } => {
                assert_eq!(x, 100.0);
                assert_eq!(y, 200.0);
                assert_eq!(label, Some("test".into()));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cursor_mode_speed_multiplier() {
        assert_eq!(CursorMode::Visible.speed_multiplier(), 1.0);
        assert_eq!(CursorMode::Fast.speed_multiplier(), 10.0);
        assert_eq!(CursorMode::Invisible.speed_multiplier(), 0.0);
    }

    #[test]
    fn test_hide_clears_state() {
        let mut state = GhostCursorState::new();
        state.show();
        state.move_to(100.0, 200.0, Some("test".into()));
        state.hide();
        assert!(!state.visible);
        assert!(state.action_label.is_none());
        assert!(state.trail.is_empty());
    }

    #[test]
    fn test_transform_style() {
        let mut state = GhostCursorState::new();
        state.move_to(150.5, 300.0, None);
        let style = state.transform_style();
        assert!(style.contains("150.5"));
        assert!(style.contains("300"));
    }
}
