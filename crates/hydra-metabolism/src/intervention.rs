//! Intervention events — actions triggered by stability degradation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The severity level of an intervention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InterventionLevel {
    /// No intervention needed.
    None,
    /// Level 1: alert the principal.
    Level1Alert,
    /// Level 2: critical — reduce load and alert.
    Level2Critical,
    /// Level 3: emergency — initiate safe shutdown or recovery.
    Level3Emergency,
}

impl std::fmt::Display for InterventionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "NONE"),
            Self::Level1Alert => write!(f, "LEVEL-1-ALERT"),
            Self::Level2Critical => write!(f, "LEVEL-2-CRITICAL"),
            Self::Level3Emergency => write!(f, "LEVEL-3-EMERGENCY"),
        }
    }
}

/// A record of an intervention event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionEvent {
    /// When the intervention was triggered.
    pub triggered_at: DateTime<Utc>,
    /// The severity level.
    pub level: InterventionLevel,
    /// The Lyapunov value that triggered the intervention.
    pub lyapunov_value: f64,
    /// Actions taken in response.
    pub actions_taken: Vec<String>,
}

impl InterventionEvent {
    /// Create a new intervention event.
    pub fn new(level: InterventionLevel, lyapunov_value: f64, actions: Vec<String>) -> Self {
        Self {
            triggered_at: Utc::now(),
            level,
            lyapunov_value,
            actions_taken: actions,
        }
    }

    /// Return a summary line for display.
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} V(Psi)={:.4} actions=[{}]",
            self.triggered_at.format("%H:%M:%S"),
            self.level,
            self.lyapunov_value,
            self.actions_taken.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(InterventionLevel::None < InterventionLevel::Level1Alert);
        assert!(InterventionLevel::Level1Alert < InterventionLevel::Level2Critical);
        assert!(InterventionLevel::Level2Critical < InterventionLevel::Level3Emergency);
    }

    #[test]
    fn event_summary_contains_level() {
        let event = InterventionEvent::new(
            InterventionLevel::Level1Alert,
            -0.1,
            vec!["notify_principal".to_string()],
        );
        let s = event.summary();
        assert!(s.contains("LEVEL-1-ALERT"));
    }
}
