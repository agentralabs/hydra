use serde::{Deserialize, Serialize};

/// Proactive updates sent from Hydra to the user during long-running operations.
/// These keep the user informed without requiring explicit polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProactiveUpdate {
    /// Immediate acknowledgment that a request was received
    Acknowledgment {
        message: String,
        estimated_duration: Option<u64>,
    },
    /// Progress report during multi-step operations
    Progress {
        percent: f32,
        current_step: String,
        steps_remaining: usize,
    },
    /// Lifecycle or noteworthy event
    Event {
        event_type: EventType,
        description: String,
        requires_attention: bool,
    },
    /// Decision point requiring user input (with timeout + default)
    Decision {
        question: String,
        options: Vec<DecisionOption>,
        timeout_secs: u64,
        default: Option<usize>,
    },
    /// Task completion with summary
    Completion {
        summary: String,
        changes: Vec<String>,
        next_steps: Vec<String>,
    },
    /// Alert requiring awareness or action
    Alert {
        severity: AlertSeverity,
        message: String,
        recoverable: bool,
        action_required: Option<String>,
    },
}

/// Types of events that can occur during execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    Started,
    PhaseChange,
    Discovery,
    Warning,
    Waiting,
    Resumed,
}

/// Severity levels for alerts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// A single option in a decision prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub label: String,
    pub description: String,
    pub keyboard_shortcut: Option<char>,
}

impl DecisionOption {
    pub fn new(label: &str, description: &str) -> Self {
        Self {
            label: label.to_string(),
            description: description.to_string(),
            keyboard_shortcut: None,
        }
    }

    pub fn with_shortcut(mut self, shortcut: char) -> Self {
        self.keyboard_shortcut = Some(shortcut);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_option_new() {
        let opt = DecisionOption::new("Yes", "Confirm the action");
        assert_eq!(opt.label, "Yes");
        assert_eq!(opt.description, "Confirm the action");
        assert!(opt.keyboard_shortcut.is_none());
    }

    #[test]
    fn test_decision_option_with_shortcut() {
        let opt = DecisionOption::new("Yes", "Confirm").with_shortcut('y');
        assert_eq!(opt.keyboard_shortcut, Some('y'));
    }

    #[test]
    fn test_event_type_serde() {
        for et in [EventType::Started, EventType::PhaseChange, EventType::Discovery, EventType::Warning, EventType::Waiting, EventType::Resumed] {
            let json = serde_json::to_string(&et).unwrap();
            let restored: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, et);
        }
    }

    #[test]
    fn test_alert_severity_serde() {
        for sev in [AlertSeverity::Info, AlertSeverity::Warning, AlertSeverity::Error, AlertSeverity::Critical] {
            let json = serde_json::to_string(&sev).unwrap();
            let restored: AlertSeverity = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, sev);
        }
    }

    #[test]
    fn test_proactive_update_acknowledgment_serde() {
        let update = ProactiveUpdate::Acknowledgment {
            message: "Got it".into(),
            estimated_duration: Some(5000),
        };
        let json = serde_json::to_string(&update).unwrap();
        let restored: ProactiveUpdate = serde_json::from_str(&json).unwrap();
        if let ProactiveUpdate::Acknowledgment { message, estimated_duration } = restored {
            assert_eq!(message, "Got it");
            assert_eq!(estimated_duration, Some(5000));
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_proactive_update_progress_serde() {
        let update = ProactiveUpdate::Progress {
            percent: 75.0,
            current_step: "Building".into(),
            steps_remaining: 2,
        };
        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("75"));
        assert!(json.contains("Building"));
    }

    #[test]
    fn test_proactive_update_completion_serde() {
        let update = ProactiveUpdate::Completion {
            summary: "All done".into(),
            changes: vec!["a.rs".into()],
            next_steps: vec!["test".into()],
        };
        let json = serde_json::to_string(&update).unwrap();
        let restored: ProactiveUpdate = serde_json::from_str(&json).unwrap();
        if let ProactiveUpdate::Completion { summary, changes, next_steps } = restored {
            assert_eq!(summary, "All done");
            assert_eq!(changes.len(), 1);
            assert_eq!(next_steps.len(), 1);
        } else {
            panic!("Wrong variant");
        }
    }
}
