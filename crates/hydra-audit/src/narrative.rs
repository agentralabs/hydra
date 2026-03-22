//! NarrativeBuilder — converts an execution trace into plain language.
//! Not logs. Not receipts. A readable account of what happened.

use crate::{
    constants::MAX_NARRATIVE_CHARS,
    errors::AuditError,
    event::EventKind,
    trace::ExecutionTrace,
};

/// The generated narrative for one execution.
#[derive(Debug, Clone)]
pub struct ExecutionNarrative {
    /// The task ID.
    pub task_id: String,
    /// The action ID.
    pub action_id: String,
    /// One-line TUI summary.
    pub summary: String,
    /// Complete narrative.
    pub full: String,
    /// Outcome label.
    pub outcome: String,
    /// Number of approaches attempted.
    pub attempt_count: usize,
    /// Number of obstacles encountered.
    pub obstacle_count: usize,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

impl ExecutionNarrative {
    /// Whether the execution completed successfully.
    pub fn is_successful(&self) -> bool {
        self.outcome == "completed"
    }
}

/// Builds plain language narratives from execution traces.
pub struct NarrativeBuilder;

impl NarrativeBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self
    }

    /// Build a narrative from an execution trace.
    pub fn build(
        &self,
        trace: &ExecutionTrace,
    ) -> Result<ExecutionNarrative, AuditError> {
        if trace.events.is_empty() {
            return Err(AuditError::EmptyTrace);
        }

        let mut lines = Vec::new();

        // Opening
        let intent = trace
            .events
            .iter()
            .find_map(|e| {
                if let EventKind::TaskStarted { intent } = &e.kind {
                    Some(intent.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| trace.action_id.clone());

        lines.push(format!("Task: \"{}\"", intent));
        lines.push(format!("Action: {}", trace.action_id));

        // Timeline
        let mut attempt_num = 0;
        for event in &trace.events {
            match &event.kind {
                EventKind::ApproachAttempted { approach } => {
                    attempt_num += 1;
                    lines.push(format!(
                        "  Attempt {}: {}",
                        attempt_num, approach
                    ));
                }
                EventKind::ObstacleEncountered { obstacle } => {
                    lines.push(format!("  → Blocked: {}", obstacle));
                }
                EventKind::Rerouting { from, to } => {
                    lines.push(format!("  → Rerouting {} → {}", from, to));
                }
                EventKind::AgentEscalated { agent_type } => {
                    lines.push(format!(
                        "  → Escalated to {} agent",
                        agent_type
                    ));
                }
                EventKind::ApproachSucceeded { approach } => {
                    lines.push(format!("  ✓ Succeeded via: {}", approach));
                }
                EventKind::TaskCompleted { duration_total_ms } => {
                    lines.push(format!(
                        "Completed in {:.1}s ({} attempt{}, {} obstacle{})",
                        *duration_total_ms as f64 / 1000.0,
                        trace.attempt_count(),
                        if trace.attempt_count() == 1 { "" } else { "s" },
                        trace.obstacle_count(),
                        if trace.obstacle_count() == 1 {
                            ""
                        } else {
                            "s"
                        },
                    ));
                }
                EventKind::TaskHardDenied { evidence } => {
                    lines.push(format!(
                        "Hard denied after {} attempt{}: {}",
                        trace.attempt_count(),
                        if trace.attempt_count() == 1 { "" } else { "s" },
                        evidence,
                    ));
                }
                _ => {}
            }
        }

        let full = lines.join("\n");
        let full = if full.len() > MAX_NARRATIVE_CHARS {
            format!(
                "{}...[truncated]",
                &full[..MAX_NARRATIVE_CHARS - 16]
            )
        } else {
            full
        };

        let outcome =
            trace.outcome().unwrap_or_else(|| "in-progress".into());

        let summary = format!(
            "[{}] {} — {} attempt{}, {} obstacle{}, {:.1}s",
            outcome.to_uppercase(),
            trace.action_id,
            trace.attempt_count(),
            if trace.attempt_count() == 1 { "" } else { "s" },
            trace.obstacle_count(),
            if trace.obstacle_count() == 1 { "" } else { "s" },
            trace.total_duration_ms() as f64 / 1000.0,
        );

        Ok(ExecutionNarrative {
            task_id: trace.task_id.clone(),
            action_id: trace.action_id.clone(),
            summary,
            full,
            outcome,
            attempt_count: trace.attempt_count(),
            obstacle_count: trace.obstacle_count(),
            duration_ms: trace.total_duration_ms(),
        })
    }
}

impl Default for NarrativeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::TraceEvent;
    use crate::trace::ExecutionTrace;

    fn make_full_trace() -> ExecutionTrace {
        let mut t = ExecutionTrace::new("t1", "deploy.staging");
        use EventKind::*;
        for (kind, receipt) in [
            (
                TaskStarted {
                    intent: "deploy to staging env".into(),
                },
                "r1",
            ),
            (
                ApproachAttempted {
                    approach: "direct".into(),
                },
                "r2",
            ),
            (
                ObstacleEncountered {
                    obstacle: "cert expired".into(),
                },
                "r3",
            ),
            (
                Rerouting {
                    from: "direct".into(),
                    to: "alternative".into(),
                },
                "r4",
            ),
            (
                ApproachAttempted {
                    approach: "alternative".into(),
                },
                "r5",
            ),
            (
                ApproachSucceeded {
                    approach: "alternative".into(),
                },
                "r6",
            ),
            (TaskCompleted { duration_total_ms: 1450 }, "r7"),
        ] {
            t.add_event(TraceEvent::new("t1", kind, receipt, 100));
        }
        t
    }

    #[test]
    fn narrative_built_from_trace() {
        let builder = NarrativeBuilder::new();
        let trace = make_full_trace();
        let narrative = builder.build(&trace).expect("build");
        assert!(narrative.is_successful());
        assert!(!narrative.full.is_empty());
        assert!(!narrative.summary.is_empty());
        assert_eq!(narrative.attempt_count, 2);
        assert_eq!(narrative.obstacle_count, 1);
    }

    #[test]
    fn summary_contains_outcome() {
        let builder = NarrativeBuilder::new();
        let trace = make_full_trace();
        let narrative = builder.build(&trace).expect("build");
        assert!(narrative.summary.contains("COMPLETED"));
    }

    #[test]
    fn narrative_contains_obstacle_description() {
        let builder = NarrativeBuilder::new();
        let trace = make_full_trace();
        let narrative = builder.build(&trace).expect("build");
        assert!(narrative.full.contains("cert expired"));
    }

    #[test]
    fn empty_trace_returns_error() {
        let builder = NarrativeBuilder::new();
        let trace = ExecutionTrace::new("t1", "action");
        assert!(matches!(
            builder.build(&trace),
            Err(AuditError::EmptyTrace)
        ));
    }
}
