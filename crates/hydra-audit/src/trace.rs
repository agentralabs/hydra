//! ExecutionTrace — the ordered timeline of events for one task.
//! Built from receipt chains. Chronologically ordered.

use crate::{
    constants::MAX_TRACE_EVENTS,
    event::TraceEvent,
};
use serde::{Deserialize, Serialize};

/// The execution trace for one task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// The task ID this trace belongs to.
    pub task_id: String,
    /// The action ID being executed.
    pub action_id: String,
    /// Ordered list of events in this execution.
    pub events: Vec<TraceEvent>,
    /// When the trace started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When the trace ended (None if still in progress).
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ExecutionTrace {
    /// Create a new empty execution trace.
    pub fn new(task_id: impl Into<String>, action_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            action_id: action_id.into(),
            events: Vec::new(),
            started_at: chrono::Utc::now(),
            ended_at: None,
        }
    }

    /// Add an event to the trace.
    pub fn add_event(&mut self, event: TraceEvent) {
        if self.events.len() < MAX_TRACE_EVENTS {
            if event.kind.is_terminal() {
                self.ended_at = Some(event.occurred_at);
            }
            self.events.push(event);
        }
    }

    /// Total duration in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        match self.ended_at {
            Some(end) => (end - self.started_at).num_milliseconds() as u64,
            None => (chrono::Utc::now() - self.started_at)
                .num_milliseconds() as u64,
        }
    }

    /// True if the trace has a terminal event.
    pub fn is_complete(&self) -> bool {
        self.events.iter().any(|e| e.kind.is_terminal())
    }

    /// The terminal outcome.
    pub fn outcome(&self) -> Option<String> {
        self.events
            .iter()
            .find(|e| e.kind.is_terminal())
            .map(|e| e.kind.label().to_string())
    }

    /// Number of approach attempts.
    pub fn attempt_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| {
                matches!(e.kind, crate::event::EventKind::ApproachAttempted { .. })
            })
            .count()
    }

    /// Number of obstacles encountered.
    pub fn obstacle_count(&self) -> usize {
        self.events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    crate::event::EventKind::ObstacleEncountered { .. }
                )
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventKind;

    fn make_trace() -> ExecutionTrace {
        let mut t = ExecutionTrace::new("task-1", "deploy.staging");
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::TaskStarted {
                intent: "deploy to staging".into(),
            },
            "receipt-1",
            0,
        ));
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::ApproachAttempted {
                approach: "direct".into(),
            },
            "receipt-2",
            100,
        ));
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::ObstacleEncountered {
                obstacle: "auth missing".into(),
            },
            "receipt-3",
            50,
        ));
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::Rerouting {
                from: "direct".into(),
                to: "alt".into(),
            },
            "receipt-4",
            10,
        ));
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::ApproachSucceeded {
                approach: "alt".into(),
            },
            "receipt-5",
            200,
        ));
        t.add_event(TraceEvent::new(
            "task-1",
            EventKind::TaskCompleted {
                duration_total_ms: 360,
            },
            "receipt-6",
            0,
        ));
        t
    }

    #[test]
    fn trace_complete_after_terminal_event() {
        let t = make_trace();
        assert!(t.is_complete());
        assert_eq!(t.outcome(), Some("completed".into()));
    }

    #[test]
    fn attempt_and_obstacle_counts() {
        let t = make_trace();
        assert_eq!(t.attempt_count(), 1);
        assert_eq!(t.obstacle_count(), 1);
    }

    #[test]
    fn event_count_correct() {
        let t = make_trace();
        assert_eq!(t.events.len(), 6);
    }
}
