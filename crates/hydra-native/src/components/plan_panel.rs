//! Plan panel component data — goal display, step list, progress bar, ETA.

use serde::{Deserialize, Serialize};

/// Status of a single plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// A single step in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub index: usize,
    pub label: String,
    pub status: StepStatus,
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
}

/// The plan panel view model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPanel {
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub eta_seconds: Option<u64>,
    pub started_at: Option<String>,
}

impl PlanPanel {
    /// Create a new plan panel with a goal and step labels.
    pub fn new(goal: &str, step_labels: Vec<&str>) -> Self {
        let steps = step_labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| PlanStep {
                index: i,
                label: label.to_owned(),
                status: StepStatus::Pending,
                detail: None,
                duration_ms: None,
            })
            .collect();
        Self {
            goal: goal.to_owned(),
            steps,
            eta_seconds: None,
            started_at: None,
        }
    }

    /// Mark the given step as running.
    pub fn start_step(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Running;
        }
    }

    /// Mark the given step as completed with optional detail and duration.
    pub fn complete_step(&mut self, index: usize, detail: Option<&str>, duration_ms: Option<u64>) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Completed;
            step.detail = detail.map(|s| s.to_owned());
            step.duration_ms = duration_ms;
        }
    }

    /// Mark the given step as failed.
    pub fn fail_step(&mut self, index: usize, reason: &str) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Failed;
            step.detail = Some(reason.to_owned());
        }
    }

    /// Mark the given step as skipped.
    pub fn skip_step(&mut self, index: usize) {
        if let Some(step) = self.steps.get_mut(index) {
            step.status = StepStatus::Skipped;
        }
    }

    /// Completion percentage (0.0..=100.0). Completed and Skipped count as done.
    pub fn progress_percent(&self) -> f32 {
        if self.steps.is_empty() {
            return 0.0;
        }
        let done = self
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed | StepStatus::Skipped))
            .count();
        (done as f32 / self.steps.len() as f32) * 100.0
    }

    /// Number of completed steps (not counting skipped).
    pub fn completed_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count()
    }

    /// The currently running step, if any.
    pub fn current_step(&self) -> Option<&PlanStep> {
        self.steps.iter().find(|s| s.status == StepStatus::Running)
    }

    /// Whether all steps are terminal (completed, failed, or skipped).
    pub fn is_finished(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(
                s.status,
                StepStatus::Completed | StepStatus::Failed | StepStatus::Skipped
            )
        })
    }

    /// Update the ETA in seconds.
    pub fn set_eta(&mut self, seconds: u64) {
        self.eta_seconds = Some(seconds);
    }

    /// Format ETA as a human-readable string.
    pub fn eta_display(&self) -> Option<String> {
        self.eta_seconds.map(|s| {
            if s < 60 {
                format!("{}s", s)
            } else if s < 3600 {
                format!("{}m {}s", s / 60, s % 60)
            } else {
                format!("{}h {}m", s / 3600, (s % 3600) / 60)
            }
        })
    }

    /// CSS class for the progress bar based on percentage.
    pub fn progress_css_class(&self) -> &'static str {
        let pct = self.progress_percent();
        if pct >= 100.0 {
            "progress-complete"
        } else if pct >= 75.0 {
            "progress-high"
        } else if pct >= 25.0 {
            "progress-mid"
        } else {
            "progress-low"
        }
    }

    /// Icon for a step based on its status.
    pub fn step_icon(status: StepStatus) -> &'static str {
        match status {
            StepStatus::Pending => "\u{25CB}",   // empty circle
            StepStatus::Running => "\u{25C9}",   // active circle
            StepStatus::Completed => "\u{2713}", // checkmark
            StepStatus::Failed => "\u{2717}",    // cross
            StepStatus::Skipped => "\u{2014}",   // em dash
        }
    }
}

impl Default for PlanPanel {
    fn default() -> Self {
        Self::new("", vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_plan() {
        let p = PlanPanel::new("Deploy app", vec!["Build", "Test", "Deploy"]);
        assert_eq!(p.goal, "Deploy app");
        assert_eq!(p.steps.len(), 3);
        assert!(p.steps.iter().all(|s| s.status == StepStatus::Pending));
        assert_eq!(p.progress_percent(), 0.0);
    }

    #[test]
    fn test_start_and_complete_step() {
        let mut p = PlanPanel::new("Task", vec!["A", "B"]);
        p.start_step(0);
        assert_eq!(p.steps[0].status, StepStatus::Running);
        assert_eq!(p.current_step().unwrap().index, 0);

        p.complete_step(0, Some("done fast"), Some(150));
        assert_eq!(p.steps[0].status, StepStatus::Completed);
        assert_eq!(p.steps[0].detail.as_deref(), Some("done fast"));
        assert_eq!(p.steps[0].duration_ms, Some(150));
        assert_eq!(p.progress_percent(), 50.0);
    }

    #[test]
    fn test_fail_step() {
        let mut p = PlanPanel::new("Task", vec!["A"]);
        p.fail_step(0, "timeout");
        assert_eq!(p.steps[0].status, StepStatus::Failed);
        assert_eq!(p.steps[0].detail.as_deref(), Some("timeout"));
    }

    #[test]
    fn test_skip_step_counts_as_done() {
        let mut p = PlanPanel::new("Task", vec!["A", "B"]);
        p.skip_step(0);
        assert_eq!(p.progress_percent(), 50.0);
        assert_eq!(p.completed_count(), 0); // skipped != completed
    }

    #[test]
    fn test_is_finished() {
        let mut p = PlanPanel::new("Task", vec!["A", "B", "C"]);
        assert!(!p.is_finished());
        p.complete_step(0, None, None);
        p.fail_step(1, "err");
        p.skip_step(2);
        assert!(p.is_finished());
    }

    #[test]
    fn test_eta_display() {
        let mut p = PlanPanel::new("Task", vec![]);
        assert!(p.eta_display().is_none());

        p.set_eta(45);
        assert_eq!(p.eta_display().as_deref(), Some("45s"));

        p.set_eta(125);
        assert_eq!(p.eta_display().as_deref(), Some("2m 5s"));

        p.set_eta(3661);
        assert_eq!(p.eta_display().as_deref(), Some("1h 1m"));
    }

    #[test]
    fn test_progress_css_class() {
        let mut p = PlanPanel::new("Task", vec!["A", "B", "C", "D"]);
        assert_eq!(p.progress_css_class(), "progress-low");

        p.complete_step(0, None, None);
        assert_eq!(p.progress_css_class(), "progress-mid");

        p.complete_step(1, None, None);
        p.complete_step(2, None, None);
        assert_eq!(p.progress_css_class(), "progress-high");

        p.complete_step(3, None, None);
        assert_eq!(p.progress_css_class(), "progress-complete");
    }

    #[test]
    fn test_step_icons() {
        assert_eq!(PlanPanel::step_icon(StepStatus::Pending), "\u{25CB}");
        assert_eq!(PlanPanel::step_icon(StepStatus::Running), "\u{25C9}");
        assert_eq!(PlanPanel::step_icon(StepStatus::Completed), "\u{2713}");
        assert_eq!(PlanPanel::step_icon(StepStatus::Failed), "\u{2717}");
        assert_eq!(PlanPanel::step_icon(StepStatus::Skipped), "\u{2014}");
    }

    #[test]
    fn test_empty_plan() {
        let p = PlanPanel::new("Empty", vec![]);
        assert_eq!(p.progress_percent(), 0.0);
        assert!(p.is_finished());
        assert!(p.current_step().is_none());
    }

    #[test]
    fn test_default() {
        let p = PlanPanel::default();
        assert_eq!(p.goal, "");
        assert!(p.steps.is_empty());
    }

    #[test]
    fn test_out_of_bounds_start_step_is_noop() {
        let mut p = PlanPanel::new("Task", vec!["A"]);
        p.start_step(99);
        assert_eq!(p.steps[0].status, StepStatus::Pending);
    }

    #[test]
    fn test_out_of_bounds_complete_step_is_noop() {
        let mut p = PlanPanel::new("Task", vec!["A"]);
        p.complete_step(99, Some("detail"), Some(100));
        assert_eq!(p.steps[0].status, StepStatus::Pending);
    }

    #[test]
    fn test_out_of_bounds_fail_step_is_noop() {
        let mut p = PlanPanel::new("Task", vec!["A"]);
        p.fail_step(99, "reason");
        assert_eq!(p.steps[0].status, StepStatus::Pending);
    }

    #[test]
    fn test_out_of_bounds_skip_step_is_noop() {
        let mut p = PlanPanel::new("Task", vec!["A"]);
        p.skip_step(99);
        assert_eq!(p.steps[0].status, StepStatus::Pending);
    }

    #[test]
    fn test_all_steps_completed_is_100_percent() {
        let mut p = PlanPanel::new("Task", vec!["A", "B", "C"]);
        p.complete_step(0, None, None);
        p.complete_step(1, None, None);
        p.complete_step(2, None, None);
        assert_eq!(p.progress_percent(), 100.0);
        assert!(p.is_finished());
    }

    #[test]
    fn test_mixed_terminal_states() {
        let mut p = PlanPanel::new("Task", vec!["A", "B", "C", "D"]);
        p.complete_step(0, None, None);
        p.fail_step(1, "err");
        p.skip_step(2);
        // Step 3 still pending
        assert!(!p.is_finished());
        // 2 done (completed + skipped) out of 4
        assert_eq!(p.progress_percent(), 50.0);
        // Only 1 truly completed
        assert_eq!(p.completed_count(), 1);
    }

    #[test]
    fn test_current_step_returns_none_when_no_running() {
        let mut p = PlanPanel::new("Task", vec!["A", "B"]);
        assert!(p.current_step().is_none());
        p.complete_step(0, None, None);
        assert!(p.current_step().is_none());
    }

    #[test]
    fn test_step_indices_are_sequential() {
        let p = PlanPanel::new("Task", vec!["A", "B", "C"]);
        for (i, step) in p.steps.iter().enumerate() {
            assert_eq!(step.index, i);
        }
    }

    #[test]
    fn test_eta_display_boundary_60_seconds() {
        let mut p = PlanPanel::new("Task", vec![]);
        p.set_eta(60);
        assert_eq!(p.eta_display().as_deref(), Some("1m 0s"));
    }

    #[test]
    fn test_eta_display_boundary_3600_seconds() {
        let mut p = PlanPanel::new("Task", vec![]);
        p.set_eta(3600);
        assert_eq!(p.eta_display().as_deref(), Some("1h 0m"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut p = PlanPanel::new("Deploy", vec!["Build", "Test", "Ship"]);
        p.start_step(0);
        p.complete_step(0, Some("built"), Some(500));
        p.set_eta(120);
        let json = serde_json::to_string(&p).unwrap();
        let back: PlanPanel = serde_json::from_str(&json).unwrap();
        assert_eq!(back.goal, "Deploy");
        assert_eq!(back.steps.len(), 3);
        assert_eq!(back.steps[0].status, StepStatus::Completed);
        assert_eq!(back.eta_seconds, Some(120));
    }

    #[test]
    fn test_step_status_serialization() {
        let statuses = [
            StepStatus::Pending,
            StepStatus::Running,
            StepStatus::Completed,
            StepStatus::Failed,
            StepStatus::Skipped,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: StepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }
}
