//! RedTeamScenario -- full adversarial simulation result.

use crate::{surface::AttackSurface, threat::ThreatVector};
use serde::{Deserialize, Serialize};

/// The complete red team analysis for one proposed action.
#[derive(Debug, Clone)]
pub struct RedTeamScenario {
    pub id: String,
    pub context: String,
    pub surfaces: Vec<AttackSurface>,
    pub threats: Vec<ThreatVector>,
    pub overall_risk: f64,
    pub go_no_go: GoNoGo,
    pub summary: String,
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
}

/// Red team recommendation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoNoGo {
    /// Proceed -- no critical threats identified.
    Go,
    /// Proceed with mitigations applied.
    GoWithMitigations { mitigations: Vec<String> },
    /// Do not proceed -- critical unmitigated threats.
    NoGo { blockers: Vec<String> },
}

impl GoNoGo {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Go => "GO",
            Self::GoWithMitigations { .. } => "GO-WITH-MITIGATIONS",
            Self::NoGo { .. } => "NO-GO",
        }
    }
}

impl RedTeamScenario {
    pub fn new(
        context: impl Into<String>,
        surfaces: Vec<AttackSurface>,
        threats: Vec<ThreatVector>,
    ) -> Self {
        let context_str = context.into();

        // Overall risk = weighted average of threat risk scores
        let overall_risk = if threats.is_empty() {
            0.0
        } else {
            threats.iter().map(|t| t.risk_score()).sum::<f64>() / threats.len() as f64
        };

        // Go/no-go decision
        let critical_threats: Vec<String> = threats
            .iter()
            .filter(|t| t.is_critical())
            .map(|t| t.name.clone())
            .collect();

        let high_threats: Vec<String> = threats
            .iter()
            .filter(|t| t.is_high() && !t.is_critical())
            .map(|t| t.mitigation.clone())
            .collect();

        let go_no_go = if !critical_threats.is_empty() {
            GoNoGo::NoGo {
                blockers: critical_threats,
            }
        } else if !high_threats.is_empty() {
            GoNoGo::GoWithMitigations {
                mitigations: high_threats,
            }
        } else {
            GoNoGo::Go
        };

        let summary = format!(
            "Red team analysis: {} surfaces, {} threats, risk={:.2}. \
             Recommendation: {}.",
            surfaces.len(),
            threats.len(),
            overall_risk,
            match &go_no_go {
                GoNoGo::Go => "GO".into(),
                GoNoGo::GoWithMitigations { mitigations } =>
                    format!("GO with {} mitigation(s)", mitigations.len()),
                GoNoGo::NoGo { blockers } =>
                    format!("NO-GO: {} critical threat(s)", blockers.len()),
            }
        );

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            context: context_str,
            surfaces,
            threats,
            overall_risk,
            go_no_go,
            summary,
            analyzed_at: chrono::Utc::now(),
        }
    }

    pub fn has_critical_threats(&self) -> bool {
        self.threats.iter().any(|t| t.is_critical())
    }

    pub fn threat_count(&self) -> usize {
        self.threats.len()
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threat::ThreatVector;

    #[test]
    fn critical_threat_triggers_no_go() {
        let critical = ThreatVector::new("Critical", "d", "g", 1.0, 1.0, vec![], "m");
        let scenario = RedTeamScenario::new("test", vec![], vec![critical]);
        assert_eq!(scenario.go_no_go.label(), "NO-GO");
        assert!(scenario.has_critical_threats());
    }

    #[test]
    fn high_threat_triggers_go_with_mitigations() {
        let high = ThreatVector::new("High", "d", "g", 0.9, 0.9, vec![], "mitigate this");
        let scenario = RedTeamScenario::new("test", vec![], vec![high]);
        assert_eq!(scenario.go_no_go.label(), "GO-WITH-MITIGATIONS");
    }

    #[test]
    fn no_threats_is_go() {
        let scenario = RedTeamScenario::new("safe action", vec![], vec![]);
        assert_eq!(scenario.go_no_go.label(), "GO");
    }
}
