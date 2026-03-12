//! Threat response actions — graduated from log to lockdown.

use super::correlator::{ThreatAssessment, ThreatLevel};

/// Response action for a detected threat.
#[derive(Debug, Clone, PartialEq)]
pub enum ThreatResponse {
    /// Low: just record it.
    Log,
    /// Medium: notify user.
    Alert(String),
    /// High: raise execution gate risk threshold.
    TightenGates,
    /// High: freeze a specific agent.
    FreezeAgent(String),
    /// Critical: halt all operations.
    Lockdown,
}

impl ThreatResponse {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Log => "Log",
            Self::Alert(_) => "Alert",
            Self::TightenGates => "TightenGates",
            Self::FreezeAgent(_) => "FreezeAgent",
            Self::Lockdown => "Lockdown",
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Log => "Threat logged for analysis.".into(),
            Self::Alert(msg) => format!("Alert: {}", msg),
            Self::TightenGates => "Execution gates tightened — all actions require approval.".into(),
            Self::FreezeAgent(id) => format!("Agent {} frozen pending investigation.", id),
            Self::Lockdown => "LOCKDOWN — all operations halted. Manual intervention required.".into(),
        }
    }
}

/// Determine the appropriate response for a threat assessment.
pub fn respond(assessment: &ThreatAssessment) -> ThreatResponse {
    match assessment.threat_level {
        ThreatLevel::None => ThreatResponse::Log,
        ThreatLevel::Low => ThreatResponse::Log,
        ThreatLevel::Medium => ThreatResponse::Alert(assessment.description.clone()),
        ThreatLevel::High => {
            // If behavioral drift from a specific agent, freeze it
            let has_drift = assessment.contributing_signals.iter()
                .any(|s| s.signal_type == super::signals::SignalType::BehavioralDrift);
            if has_drift {
                let agent = assessment.contributing_signals.iter()
                    .find(|s| s.signal_type == super::signals::SignalType::BehavioralDrift)
                    .map(|s| s.source.to_string())
                    .unwrap_or_else(|| "unknown".into());
                ThreatResponse::FreezeAgent(agent)
            } else {
                ThreatResponse::TightenGates
            }
        }
        ThreatLevel::Critical => ThreatResponse::Lockdown,
    }
}

/// Format a response for display.
pub fn format_response(response: &ThreatResponse) -> String {
    match response {
        ThreatResponse::Log => "Action: Logged".into(),
        ThreatResponse::Alert(msg) => format!("Action: Alert — {}", msg),
        ThreatResponse::TightenGates => "Action: Gates tightened to maximum scrutiny".into(),
        ThreatResponse::FreezeAgent(id) => format!("Action: Agent {} frozen", id),
        ThreatResponse::Lockdown => "Action: FULL LOCKDOWN INITIATED".into(),
    }
}
