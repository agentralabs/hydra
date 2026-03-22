//! ThreatModel -- what an intelligent attacker would do given this action.

use crate::constants::*;
use hydra_axiom::primitives::AxiomPrimitive;
use serde::{Deserialize, Serialize};

/// A threat vector identified through red team analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatVector {
    pub id: String,
    pub name: String,
    pub description: String,
    pub attacker_goal: String,
    pub severity: f64,
    pub likelihood: f64,
    pub primitives: Vec<String>,
    pub mitigation: String,
}

impl ThreatVector {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        attacker_goal: impl Into<String>,
        severity: f64,
        likelihood: f64,
        primitives: Vec<String>,
        mitigation: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            attacker_goal: attacker_goal.into(),
            severity: severity.clamp(0.0, 1.0),
            likelihood: likelihood.clamp(0.0, 1.0),
            primitives,
            mitigation: mitigation.into(),
        }
    }

    pub fn risk_score(&self) -> f64 {
        self.severity * self.likelihood
    }

    pub fn is_critical(&self) -> bool {
        self.risk_score() >= CRITICAL_SEVERITY_THRESHOLD
    }

    pub fn is_high(&self) -> bool {
        self.risk_score() >= HIGH_SEVERITY_THRESHOLD
    }

    pub fn severity_label(&self) -> &'static str {
        if self.is_critical() {
            "CRITICAL"
        } else if self.is_high() {
            "HIGH"
        } else if self.risk_score() >= 0.5 {
            "MEDIUM"
        } else {
            "LOW"
        }
    }
}

/// Build threat vectors from axiom primitives.
pub fn threats_from_primitives(context: &str, primitives: &[AxiomPrimitive]) -> Vec<ThreatVector> {
    let mut threats = Vec::new();
    let lower = context.to_lowercase();

    let has_trust = primitives
        .iter()
        .any(|p| matches!(p, AxiomPrimitive::TrustRelation));
    let has_risk = primitives.iter().any(|p| matches!(p, AxiomPrimitive::Risk));
    let has_dep = primitives
        .iter()
        .any(|p| matches!(p, AxiomPrimitive::Dependency));
    let has_causal = primitives
        .iter()
        .any(|p| matches!(p, AxiomPrimitive::CausalLink));
    let has_auth = lower.contains("auth")
        || lower.contains("token")
        || lower.contains("cert")
        || lower.contains("credential");
    let has_deploy = lower.contains("deploy")
        || lower.contains("release")
        || lower.contains("ship")
        || lower.contains("publish");
    let has_api = lower.contains("api") || lower.contains("endpoint") || lower.contains("service");

    // Auth exploitation
    if has_auth && has_risk {
        threats.push(ThreatVector::new(
            "Credential Exploitation",
            "Attacker targets authentication mechanism during vulnerable window",
            "Gain unauthorized access via credential theft or replay",
            0.85,
            0.75,
            vec!["trust".into(), "risk".into()],
            "Rotate credentials before deployment. \
             Use short-lived tokens. Enable anomaly detection on auth events.",
        ));
    }

    // Supply chain attack
    if has_dep && (has_trust || has_risk) {
        threats.push(ThreatVector::new(
            "Supply Chain Attack",
            "Attacker compromises a dependency to gain execution in the target",
            "Execute arbitrary code via compromised package",
            0.80,
            0.55,
            vec!["dependency".into(), "trust".into()],
            "Pin dependency versions with hash verification. \
             Use a private artifact registry. Audit dependency graph.",
        ));
    }

    // Deployment window timing attack
    if has_deploy && has_causal {
        threats.push(ThreatVector::new(
            "Deployment Window Timing Attack",
            "Attacker exploits the brief vulnerability window during deployment \
             when old and new versions coexist",
            "Exploit version inconsistency during rollout",
            0.70,
            0.60,
            vec!["causal".into(), "risk".into()],
            "Blue-green deployment. \
             Minimize deployment window. Monitor for anomalous traffic during rollout.",
        ));
    }

    // API surface exposure
    if has_api && has_trust {
        threats.push(ThreatVector::new(
            "API Surface Enumeration",
            "Attacker maps the API surface to find undocumented or over-permissioned endpoints",
            "Discover attack surface and escalate via under-protected endpoints",
            0.65,
            0.70,
            vec!["trust".into(), "risk".into()],
            "Enforce authentication on all endpoints. \
             Rate limit discovery patterns. Monitor for scanning behavior.",
        ));
    }

    // Generic trust escalation (always relevant)
    if has_trust {
        threats.push(ThreatVector::new(
            "Privilege Escalation via Trust Chain",
            "Attacker exploits trust relationships to gain elevated permissions",
            "Escalate from low to high privilege via transitive trust",
            0.75,
            0.55,
            vec!["trust".into()],
            "Audit trust chain. Enforce least-privilege. \
             Regular permission reviews.",
        ));
    }

    // Keep only threats above minimum confidence
    threats.retain(|t| t.risk_score() >= MIN_THREAT_CONFIDENCE);

    // Sort by risk score descending
    threats.sort_by(|a, b| {
        b.risk_score()
            .partial_cmp(&a.risk_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    threats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_risk_generates_credential_threat() {
        let prims = vec![AxiomPrimitive::TrustRelation, AxiomPrimitive::Risk];
        let threats = threats_from_primitives("auth token deployment", &prims);
        assert!(threats.iter().any(|t| t.name.contains("Credential")));
    }

    #[test]
    fn dependency_risk_generates_supply_chain_threat() {
        let prims = vec![AxiomPrimitive::Dependency, AxiomPrimitive::TrustRelation];
        let threats = threats_from_primitives("package dependency install", &prims);
        assert!(threats.iter().any(|t| t.name.contains("Supply Chain")));
    }

    #[test]
    fn threats_sorted_by_risk_score() {
        let prims = vec![
            AxiomPrimitive::TrustRelation,
            AxiomPrimitive::Risk,
            AxiomPrimitive::Dependency,
            AxiomPrimitive::CausalLink,
        ];
        let threats = threats_from_primitives("auth deploy api token cert", &prims);
        for w in threats.windows(2) {
            assert!(w[0].risk_score() >= w[1].risk_score());
        }
    }

    #[test]
    fn severity_labels_correct() {
        let critical = ThreatVector::new("c", "d", "g", 1.0, 1.0, vec![], "m");
        let low = ThreatVector::new("c", "d", "g", 0.1, 0.1, vec![], "m");
        assert_eq!(critical.severity_label(), "CRITICAL");
        assert_eq!(low.severity_label(), "LOW");
    }
}
