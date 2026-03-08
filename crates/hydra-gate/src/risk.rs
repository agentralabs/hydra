use hydra_core::types::{Action, ActionType, RiskAssessment, RiskFactor, RiskLevel};
use serde::{Deserialize, Serialize};

/// Blast radius of an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlastRadius {
    Local,
    Project,
    System,
    External,
    Social,
    Financial,
}

/// Context for risk assessment
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub target_path: Option<String>,
    pub is_hydra_internal: bool,
    pub in_sandbox: bool,
    pub has_backup: bool,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            target_path: None,
            is_hydra_internal: false,
            in_sandbox: true,
            has_backup: false,
        }
    }
}

/// Rule-based risk assessor — 0 tokens, < 50ms
pub struct RiskAssessor {
    /// Paths that are always blocked (hydra internals, system files)
    blocked_paths: Vec<String>,
    /// Paths that require extra caution
    sensitive_paths: Vec<String>,
}

impl RiskAssessor {
    pub fn new() -> Self {
        Self {
            blocked_paths: vec![
                "/etc/".into(),
                "/usr/".into(),
                "/System/".into(),
                ".hydra/".into(),
                "hydra-gate".into(),
                "hydra-core".into(),
                "hydra-kernel".into(),
            ],
            sensitive_paths: vec![
                ".env".into(),
                "credentials".into(),
                "secret".into(),
                ".ssh/".into(),
                ".aws/".into(),
                "password".into(),
                "token".into(),
            ],
        }
    }

    /// Fast rule-based risk assessment (0 tokens, < 50ms)
    pub fn assess_risk_fast(&self, action: &Action, context: &ActionContext) -> RiskAssessment {
        let mut factors = Vec::new();
        let mut risk_score: f64 = 0.0;

        // Factor 1: Action type base risk (dominant factor — 60% weight)
        let type_risk = self.action_type_risk(&action.action_type);
        factors.push(RiskFactor {
            name: "action_type".into(),
            severity: score_to_level(type_risk),
            description: format!(
                "Action type {:?} has base risk {:.2}",
                action.action_type, type_risk
            ),
        });
        risk_score += type_risk * 0.6;

        // Factor 2: Target path sensitivity (25% weight)
        let path_risk = self.path_risk(&action.target, context);
        if path_risk > 0.0 {
            factors.push(RiskFactor {
                name: "target_path".into(),
                severity: score_to_level(path_risk),
                description: format!(
                    "Target '{}' has sensitivity risk {:.2}",
                    action.target, path_risk
                ),
            });
        }
        risk_score += path_risk * 0.25;

        // Factor 3: Self-modification check (EC-EG-010)
        if context.is_hydra_internal || self.is_self_modification(&action.target) {
            factors.push(RiskFactor {
                name: "self_modification".into(),
                severity: RiskLevel::Critical,
                description: "Action targets Hydra's own configuration or code.".into(),
            });
            risk_score = 0.95; // Force critical
        }

        // Factor 4: Sandbox status
        if !context.in_sandbox {
            factors.push(RiskFactor {
                name: "no_sandbox".into(),
                severity: RiskLevel::Medium,
                description: "Action runs outside sandbox.".into(),
            });
            risk_score += 0.15;
        }

        // Factor 5: Reversibility
        let reversible = self.is_reversible(&action.action_type, context);
        if !reversible {
            factors.push(RiskFactor {
                name: "irreversible".into(),
                severity: RiskLevel::High,
                description: "Action cannot be easily undone.".into(),
            });
            risk_score += 0.1;
        }

        // Factor 6: Explicitly set risk on action
        if action.risk != RiskLevel::None {
            let explicit = level_to_score(action.risk);
            risk_score = risk_score.max(explicit);
        }

        risk_score = risk_score.clamp(0.0, 1.0);
        let level = score_to_level(risk_score);

        RiskAssessment {
            level,
            factors,
            mitigations: self.suggest_mitigations(risk_score, &action.action_type),
            requires_approval: risk_score >= 0.5,
        }
    }

    /// Get the risk score for this assessment (for threshold checks)
    pub fn risk_score(assessment: &RiskAssessment) -> f64 {
        level_to_score(assessment.level)
    }

    fn action_type_risk(&self, action_type: &ActionType) -> f64 {
        match action_type {
            ActionType::Read => 0.05,
            ActionType::Write => 0.3,
            ActionType::FileCreate => 0.2,
            ActionType::FileModify => 0.35,
            ActionType::FileDelete => 0.65,
            ActionType::Execute => 0.5,
            ActionType::ShellExecute => 0.7,
            ActionType::Network => 0.4,
            ActionType::System => 0.8,
            ActionType::GitOperation => 0.3,
            ActionType::ApiCall => 0.35,
            ActionType::SisterCall => 0.1,
            ActionType::Composite => 0.5,
        }
    }

    fn path_risk(&self, target: &str, _context: &ActionContext) -> f64 {
        let lower = target.to_lowercase();

        // Blocked paths — maximum risk
        if self.blocked_paths.iter().any(|p| lower.contains(p)) {
            return 0.95;
        }

        // Sensitive paths — high risk
        if self.sensitive_paths.iter().any(|p| lower.contains(p)) {
            return 0.7;
        }

        0.0
    }

    fn is_self_modification(&self, target: &str) -> bool {
        let lower = target.to_lowercase();
        lower.contains(".hydra") || lower.contains("hydra-gate") || lower.contains("hydra-core")
    }

    fn is_reversible(&self, action_type: &ActionType, context: &ActionContext) -> bool {
        match action_type {
            ActionType::Read | ActionType::SisterCall => true,
            ActionType::FileCreate | ActionType::FileModify => true,
            ActionType::FileDelete => context.has_backup,
            ActionType::ShellExecute | ActionType::System => false,
            ActionType::Network | ActionType::ApiCall => false,
            _ => false,
        }
    }

    fn suggest_mitigations(&self, score: f64, action_type: &ActionType) -> Vec<String> {
        let mut mitigations = Vec::new();
        if score > 0.5 {
            mitigations.push("Create a backup before proceeding.".into());
        }
        if matches!(action_type, ActionType::FileDelete) {
            mitigations.push("Move to trash instead of permanent delete.".into());
        }
        if matches!(action_type, ActionType::ShellExecute) {
            mitigations.push("Run in sandboxed environment.".into());
        }
        mitigations
    }
}

impl Default for RiskAssessor {
    fn default() -> Self {
        Self::new()
    }
}

fn score_to_level(score: f64) -> RiskLevel {
    if score >= 0.9 {
        RiskLevel::Critical
    } else if score >= 0.7 {
        RiskLevel::High
    } else if score >= 0.5 {
        RiskLevel::Medium
    } else if score >= 0.3 {
        RiskLevel::Low
    } else {
        RiskLevel::None
    }
}

fn level_to_score(level: RiskLevel) -> f64 {
    match level {
        RiskLevel::None => 0.1,
        RiskLevel::Low => 0.35,
        RiskLevel::Medium => 0.55,
        RiskLevel::High => 0.75,
        RiskLevel::Critical => 0.95,
    }
}
