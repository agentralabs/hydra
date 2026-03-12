use std::time::Duration;

use serde::{Deserialize, Serialize};

use hydra_core::types::RiskLevel;

/// Gate decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    /// Risk < 0.3: proceed without user interaction
    AutoApprove { risk_score: f64 },
    /// Risk 0.3-0.5: notify user but proceed
    NotifyOnly { risk_score: f64, message: String },
    /// Risk 0.5-0.9: require explicit user approval
    RequireApproval { risk_score: f64, reason: String },
    /// Risk >= 0.9: block entirely
    Block { risk_score: f64, reason: String },
    /// User timed out — safe default applied
    TimedOut { used_default: bool },
    /// User disconnected during approval
    Aborted { reason: String },
    /// Kill switch engaged
    Halted { reason: String },
}

impl GateDecision {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::AutoApprove { .. } | Self::NotifyOnly { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Block { .. } | Self::Halted { .. })
    }

    pub fn needs_approval(&self) -> bool {
        matches!(self, Self::RequireApproval { .. })
    }

    pub fn timed_out(&self) -> bool {
        matches!(self, Self::TimedOut { .. })
    }

    pub fn used_default(&self) -> bool {
        matches!(self, Self::TimedOut { used_default: true })
    }

    pub fn aborted(&self) -> bool {
        matches!(self, Self::Aborted { .. } | Self::Halted { .. })
    }

    pub fn risk_score(&self) -> f64 {
        match self {
            Self::AutoApprove { risk_score } => *risk_score,
            Self::NotifyOnly { risk_score, .. } => *risk_score,
            Self::RequireApproval { risk_score, .. } => *risk_score,
            Self::Block { risk_score, .. } => *risk_score,
            _ => 0.0,
        }
    }

    pub fn risk_level(&self) -> RiskLevel {
        let score = self.risk_score();
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

    pub fn decision_name(&self) -> &'static str {
        match self {
            Self::AutoApprove { .. } => "auto_approve",
            Self::NotifyOnly { .. } => "notify_only",
            Self::RequireApproval { .. } => "require_approval",
            Self::Block { .. } => "block",
            Self::TimedOut { .. } => "timed_out",
            Self::Aborted { .. } => "aborted",
            Self::Halted { .. } => "halted",
        }
    }
}

/// Gate configuration
#[derive(Debug, Clone)]
pub struct GateConfig {
    pub auto_approve_below: f64,
    pub notify_below: f64,
    pub require_approval_below: f64,
    pub block_above: f64,
    pub approval_timeout: Duration,
    pub max_approval_retries: u32,
    pub shadow_sim_enabled: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            auto_approve_below: 0.3,
            notify_below: 0.5,
            require_approval_below: 0.9,
            block_above: 0.9,
            approval_timeout: Duration::from_secs(30),
            max_approval_retries: 3,
            shadow_sim_enabled: false,
        }
    }
}

/// Batch evaluation result
pub struct BatchResult {
    pub decisions: Vec<(usize, GateDecision)>,
}

impl BatchResult {
    pub fn needs_approval_for(&self, index: usize) -> bool {
        self.decisions
            .iter()
            .any(|(i, d)| *i == index && d.needs_approval())
    }
}
