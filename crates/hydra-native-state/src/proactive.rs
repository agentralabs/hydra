//! Proactive notification engine for Hydra desktop.
//!
//! Monitors triggers and generates proactive updates that surface
//! as notifications or context in the next conversation.

/// Proactive update for the UI
#[derive(Debug, Clone)]
pub struct ProactiveAlert {
    pub title: String,
    pub message: String,
    pub priority: AlertPriority,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertPriority {
    Low,
    Medium,
    High,
}

/// Simple proactive engine that queues alerts
pub struct ProactiveNotifier {
    alerts: Vec<ProactiveAlert>,
    enabled: bool,
}

impl ProactiveNotifier {
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Add a proactive alert
    pub fn push(&mut self, alert: ProactiveAlert) {
        if self.enabled {
            self.alerts.push(alert);
        }
    }

    /// Drain all pending alerts
    pub fn drain(&mut self) -> Vec<ProactiveAlert> {
        std::mem::take(&mut self.alerts)
    }

    /// Check if there are pending alerts
    pub fn has_alerts(&self) -> bool {
        !self.alerts.is_empty()
    }

    /// Generate alerts from dream insights
    pub fn from_dream_insights(&mut self, insights: &str) {
        if !insights.is_empty() {
            self.push(ProactiveAlert {
                title: "Dream Insights Available".to_string(),
                message: insights.to_string(),
                priority: AlertPriority::Low,
                suggested_action: Some("Review insights from idle processing".to_string()),
            });
        }
    }

    /// Generate proactive alerts by scanning conversation context for universal
    /// developer-workflow keywords (errors, deployments, test failures).
    ///
    /// These triggers run locally without an LLM call. The keywords are intentionally
    /// broad and language-agnostic — they match universal software engineering terms
    /// so the system can surface helpful suggestions before the user explicitly asks.
    pub fn anticipate(&mut self, context: &str) {
        let lower = context.to_lowercase();

        if lower.contains("error") || lower.contains("failed") || lower.contains("bug") {
            self.push(ProactiveAlert {
                title: "Debugging Assistance Ready".to_string(),
                message: "I detected error-related context. Want me to help debug?".to_string(),
                priority: AlertPriority::Medium,
                suggested_action: Some("Analyze error and suggest fix".to_string()),
            });
        }

        if lower.contains("deploy") || lower.contains("production") || lower.contains("release") {
            self.push(ProactiveAlert {
                title: "Pre-Deployment Check".to_string(),
                message: "Deployment detected. Should I run pre-flight checks?".to_string(),
                priority: AlertPriority::High,
                suggested_action: Some("Run tests, check dependencies, validate config".to_string()),
            });
        }

        if lower.contains("test") && (lower.contains("fail") || lower.contains("broke")) {
            self.push(ProactiveAlert {
                title: "Test Failure Analysis".to_string(),
                message: "Test failures detected. Want me to analyze the failures?".to_string(),
                priority: AlertPriority::Medium,
                suggested_action: Some("Analyze test output and suggest fixes".to_string()),
            });
        }
    }
}
