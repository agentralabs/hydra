//! Tests for the TimelinePanel.

#[cfg(test)]
mod tests {
    use crate::components::timeline_panel::{TimelineEventKind, TimelinePanel};

    #[test]
    fn test_new_timeline() {
        let tp = TimelinePanel::new();
        assert!(tp.events.is_empty());
        assert!(tp.auto_scroll);
        assert!(tp.filter.is_none());
    }

    #[test]
    fn test_push_event() {
        let mut tp = TimelinePanel::new();
        let id = tp.push_event(
            "12:00:01",
            TimelineEventKind::Info,
            "Started",
            Some("Initializing"),
            None,
        );
        assert_eq!(id, 0);
        assert_eq!(tp.event_count(), 1);
        assert_eq!(tp.events[0].title, "Started");
        assert_eq!(tp.events[0].timestamp, "12:00:01");
    }

    #[test]
    fn test_sequential_ids() {
        let mut tp = TimelinePanel::new();
        let id0 = tp.push_event("t0", TimelineEventKind::Info, "A", None, None);
        let id1 = tp.push_event("t1", TimelineEventKind::Info, "B", None, None);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
    }

    #[test]
    fn test_set_event_duration() {
        let mut tp = TimelinePanel::new();
        let id = tp.push_event("t0", TimelineEventKind::ToolCall, "Read file", None, None);
        assert!(tp.events[0].duration_ms.is_none());
        tp.set_event_duration(id, 250);
        assert_eq!(tp.events[0].duration_ms, Some(250));
    }

    #[test]
    fn test_filter_events() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "A", None, None);
        tp.push_event("t1", TimelineEventKind::Error, "B", None, None);
        tp.push_event("t2", TimelineEventKind::Info, "C", None, None);

        assert_eq!(tp.visible_events().len(), 3);

        tp.set_filter(Some(TimelineEventKind::Error));
        let visible = tp.visible_events();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].title, "B");

        tp.set_filter(None);
        assert_eq!(tp.visible_events().len(), 3);
    }

    #[test]
    fn test_error_count() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Error, "Err1", None, None);
        tp.push_event("t1", TimelineEventKind::Info, "Ok", None, None);
        tp.push_event("t2", TimelineEventKind::Error, "Err2", None, None);
        assert_eq!(tp.error_count(), 2);
    }

    #[test]
    fn test_latest_event() {
        let mut tp = TimelinePanel::new();
        assert!(tp.latest_event().is_none());
        tp.push_event("t0", TimelineEventKind::Info, "First", None, None);
        tp.push_event("t1", TimelineEventKind::Info, "Second", None, None);
        assert_eq!(tp.latest_event().unwrap().title, "Second");
    }

    #[test]
    fn test_clear() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "A", None, None);
        tp.push_event("t1", TimelineEventKind::Info, "B", None, None);
        tp.clear();
        assert!(tp.events.is_empty());
        assert_eq!(tp.event_count(), 0);
        // IDs reset
        let id = tp.push_event("t2", TimelineEventKind::Info, "C", None, None);
        assert_eq!(id, 0);
    }

    #[test]
    fn test_toggle_auto_scroll() {
        let mut tp = TimelinePanel::new();
        assert!(tp.auto_scroll);
        tp.toggle_auto_scroll();
        assert!(!tp.auto_scroll);
        tp.toggle_auto_scroll();
        assert!(tp.auto_scroll);
    }

    #[test]
    fn test_phase_label() {
        let mut tp = TimelinePanel::new();
        tp.push_event(
            "t0",
            TimelineEventKind::PhaseChange,
            "Perceive started",
            None,
            Some("Perceive"),
        );
        assert_eq!(tp.events[0].phase_label.as_deref(), Some("Perceive"));
    }

    #[test]
    fn test_event_css_classes() {
        assert_eq!(
            TimelinePanel::event_css_class(TimelineEventKind::PhaseChange),
            "timeline-phase"
        );
        assert_eq!(
            TimelinePanel::event_css_class(TimelineEventKind::Error),
            "timeline-error"
        );
        assert_eq!(
            TimelinePanel::event_css_class(TimelineEventKind::Delegation),
            "timeline-delegation"
        );
    }

    #[test]
    fn test_event_icons() {
        // Verify all variants return non-empty strings
        let kinds = [
            TimelineEventKind::PhaseChange,
            TimelineEventKind::ToolCall,
            TimelineEventKind::Approval,
            TimelineEventKind::Error,
            TimelineEventKind::Info,
            TimelineEventKind::Delegation,
        ];
        for kind in kinds {
            assert!(!TimelinePanel::event_icon(kind).is_empty());
        }
    }

    #[test]
    fn test_set_duration_nonexistent_id_is_noop() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "A", None, None);
        tp.set_event_duration(999, 100);
        assert!(tp.events[0].duration_ms.is_none());
    }

    #[test]
    fn test_filter_with_no_matches() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "A", None, None);
        tp.push_event("t1", TimelineEventKind::Info, "B", None, None);
        tp.set_filter(Some(TimelineEventKind::Error));
        assert!(tp.visible_events().is_empty());
    }

    #[test]
    fn test_event_detail_preserved() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::ToolCall, "Read", Some("file.rs"), None);
        assert_eq!(tp.events[0].detail.as_deref(), Some("file.rs"));
    }

    #[test]
    fn test_all_css_classes_unique() {
        let kinds = [
            TimelineEventKind::PhaseChange,
            TimelineEventKind::ToolCall,
            TimelineEventKind::Approval,
            TimelineEventKind::Error,
            TimelineEventKind::Info,
            TimelineEventKind::Delegation,
        ];
        let classes: Vec<&str> = kinds.iter().map(|k| TimelinePanel::event_css_class(*k)).collect();
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_all_icons_unique() {
        let kinds = [
            TimelineEventKind::PhaseChange,
            TimelineEventKind::ToolCall,
            TimelineEventKind::Approval,
            TimelineEventKind::Error,
            TimelineEventKind::Info,
            TimelineEventKind::Delegation,
        ];
        let icons: Vec<&str> = kinds.iter().map(|k| TimelinePanel::event_icon(*k)).collect();
        for (i, a) in icons.iter().enumerate() {
            for (j, b) in icons.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "Start", Some("init"), Some("Perceive"));
        let json = serde_json::to_string(&tp).unwrap();
        let back: TimelinePanel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 1);
        assert_eq!(back.events[0].title, "Start");
        assert_eq!(back.events[0].phase_label.as_deref(), Some("Perceive"));
    }

    #[test]
    fn test_event_kind_serialization() {
        let kinds = [
            TimelineEventKind::PhaseChange,
            TimelineEventKind::ToolCall,
            TimelineEventKind::Approval,
            TimelineEventKind::Error,
            TimelineEventKind::Info,
            TimelineEventKind::Delegation,
        ];
        for k in &kinds {
            let json = serde_json::to_string(k).unwrap();
            let back: TimelineEventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(*k, back);
        }
    }

    #[test]
    fn test_many_events_ordering() {
        let mut tp = TimelinePanel::new();
        for i in 0..50 {
            tp.push_event(&format!("t{}", i), TimelineEventKind::Info, &format!("Event {}", i), None, None);
        }
        assert_eq!(tp.event_count(), 50);
        assert_eq!(tp.events[0].title, "Event 0");
        assert_eq!(tp.events[49].title, "Event 49");
        assert_eq!(tp.latest_event().unwrap().title, "Event 49");
    }

    #[test]
    fn test_error_count_with_no_errors() {
        let mut tp = TimelinePanel::new();
        tp.push_event("t0", TimelineEventKind::Info, "Ok", None, None);
        tp.push_event("t1", TimelineEventKind::ToolCall, "Read", None, None);
        assert_eq!(tp.error_count(), 0);
    }
}
