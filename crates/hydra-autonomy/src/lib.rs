//! hydra-autonomy — Graduated autonomy for Hydra.
//!
//! Trust scores determine what Hydra can do autonomously.
//! Trust is earned through successful actions and decays over time
//! or after failures. Higher trust unlocks higher autonomy levels.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Autonomy levels — what Hydra is allowed to do without asking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Must ask permission for everything
    Observer,
    /// Can read and analyze, must ask before any writes
    Apprentice,
    /// Can perform safe writes (e.g. creating files in safe locations)
    Assistant,
    /// Can perform most actions, asks only for destructive/irreversible ones
    Partner,
    /// Full autonomy (still respects hard safety constraints)
    Autonomous,
}

impl AutonomyLevel {
    /// Get the minimum trust score required for this level
    pub fn required_trust(&self) -> f64 {
        match self {
            Self::Observer => 0.0,
            Self::Apprentice => 0.2,
            Self::Assistant => 0.4,
            Self::Partner => 0.7,
            Self::Autonomous => 0.9,
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Observer => "Read-only — asks permission for everything",
            Self::Apprentice => "Can read and analyze, asks before writing",
            Self::Assistant => "Can perform safe actions autonomously",
            Self::Partner => "Autonomous except for destructive actions",
            Self::Autonomous => "Full autonomy within safety constraints",
        }
    }
}

/// Risk level of an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionRisk {
    /// No risk (e.g. reading a file)
    None,
    /// Low risk (e.g. creating a file in a safe location)
    Low,
    /// Medium risk (e.g. modifying a file)
    Medium,
    /// High risk (e.g. deleting files, running arbitrary commands)
    High,
    /// Critical risk (e.g. deploying to production, deleting databases)
    Critical,
}

impl ActionRisk {
    /// Minimum autonomy level needed to perform this action without asking
    pub fn required_autonomy(&self) -> AutonomyLevel {
        match self {
            Self::None => AutonomyLevel::Observer,
            Self::Low => AutonomyLevel::Apprentice,
            Self::Medium => AutonomyLevel::Partner,
            Self::High => AutonomyLevel::Autonomous,
            Self::Critical => AutonomyLevel::Autonomous, // Always checked even at max
        }
    }
}

/// Trust domain — trust can vary by domain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrustDomain(pub String);

impl TrustDomain {
    pub fn global() -> Self {
        Self("global".into())
    }

    pub fn new(name: &str) -> Self {
        Self(name.into())
    }
}

/// A trust score with history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub value: f64,
    pub domain: TrustDomain,
    pub total_actions: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub last_updated: String,
    pub history: Vec<TrustEvent>,
}

/// An event that changed the trust score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvent {
    pub delta: f64,
    pub reason: String,
    pub timestamp: String,
    pub new_value: f64,
}

impl TrustScore {
    pub fn new(domain: TrustDomain) -> Self {
        Self {
            value: 0.0,
            domain,
            total_actions: 0,
            successful_actions: 0,
            failed_actions: 0,
            last_updated: chrono::Utc::now().to_rfc3339(),
            history: Vec::new(),
        }
    }

    /// Get the current autonomy level based on trust
    pub fn autonomy_level(&self) -> AutonomyLevel {
        if self.value >= AutonomyLevel::Autonomous.required_trust() {
            AutonomyLevel::Autonomous
        } else if self.value >= AutonomyLevel::Partner.required_trust() {
            AutonomyLevel::Partner
        } else if self.value >= AutonomyLevel::Assistant.required_trust() {
            AutonomyLevel::Assistant
        } else if self.value >= AutonomyLevel::Apprentice.required_trust() {
            AutonomyLevel::Apprentice
        } else {
            AutonomyLevel::Observer
        }
    }

    /// Record a successful action — earns trust
    fn earn(&mut self, amount: f64, reason: &str) {
        let delta = amount.clamp(0.0, 0.1); // Max +0.1 per action
        self.value = (self.value + delta).clamp(0.0, 1.0);
        self.total_actions += 1;
        self.successful_actions += 1;
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.history.push(TrustEvent {
            delta,
            reason: reason.into(),
            timestamp: self.last_updated.clone(),
            new_value: self.value,
        });
    }

    /// Record a failed action — loses trust
    fn penalize(&mut self, amount: f64, reason: &str) {
        let delta = -(amount.clamp(0.0, 0.3)); // Max -0.3 per failure
        self.value = (self.value + delta).clamp(0.0, 1.0);
        self.total_actions += 1;
        self.failed_actions += 1;
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.history.push(TrustEvent {
            delta,
            reason: reason.into(),
            timestamp: self.last_updated.clone(),
            new_value: self.value,
        });
    }

    /// Decay trust over time
    fn decay(&mut self, factor: f64) {
        let old = self.value;
        self.value *= factor.clamp(0.0, 1.0);
        if (old - self.value).abs() > 0.001 {
            self.last_updated = chrono::Utc::now().to_rfc3339();
            self.history.push(TrustEvent {
                delta: self.value - old,
                reason: "time decay".into(),
                timestamp: self.last_updated.clone(),
                new_value: self.value,
            });
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_actions == 0 {
            return 0.0;
        }
        self.successful_actions as f64 / self.total_actions as f64
    }
}

/// Decision on whether to allow an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyDecision {
    pub allowed: bool,
    pub requires_approval: bool,
    pub autonomy_level: AutonomyLevel,
    pub trust_score: f64,
    pub action_risk: ActionRisk,
    pub reason: String,
}

/// Graduated autonomy manager
pub struct GraduatedAutonomy {
    trust_scores: parking_lot::RwLock<HashMap<TrustDomain, TrustScore>>,
    decisions: parking_lot::RwLock<Vec<AutonomyDecision>>,
    /// Hard ceiling: never exceed this autonomy level regardless of trust
    max_autonomy: AutonomyLevel,
    /// Earn rate multiplier
    earn_rate: f64,
    /// Penalty rate multiplier
    penalty_rate: f64,
    /// Decay factor (applied periodically)
    decay_factor: f64,
}

impl GraduatedAutonomy {
    pub fn new(max_autonomy: AutonomyLevel) -> Self {
        let mut trust_scores = HashMap::new();
        trust_scores.insert(TrustDomain::global(), TrustScore::new(TrustDomain::global()));

        Self {
            trust_scores: parking_lot::RwLock::new(trust_scores),
            decisions: parking_lot::RwLock::new(Vec::new()),
            max_autonomy,
            earn_rate: 1.0,
            penalty_rate: 1.0,
            decay_factor: 0.99,
        }
    }

    /// Set earn rate multiplier
    pub fn with_earn_rate(mut self, rate: f64) -> Self {
        self.earn_rate = rate.clamp(0.1, 5.0);
        self
    }

    /// Set penalty rate multiplier
    pub fn with_penalty_rate(mut self, rate: f64) -> Self {
        self.penalty_rate = rate.clamp(0.1, 5.0);
        self
    }

    /// Set decay factor
    pub fn with_decay_factor(mut self, factor: f64) -> Self {
        self.decay_factor = factor.clamp(0.8, 1.0);
        self
    }

    /// Check if an action is allowed at the current trust level
    pub fn check_action(&self, domain: &TrustDomain, risk: ActionRisk) -> AutonomyDecision {
        let scores = self.trust_scores.read();
        let score = scores
            .get(domain)
            .or_else(|| scores.get(&TrustDomain::global()));

        let (trust_value, current_level) = match score {
            Some(s) => (s.value, s.autonomy_level()),
            None => (0.0, AutonomyLevel::Observer),
        };

        // Apply ceiling
        let effective_level = current_level.min(self.max_autonomy);
        let required_level = risk.required_autonomy();

        let allowed = effective_level >= required_level;
        let requires_approval = !allowed || risk == ActionRisk::Critical;

        let reason = if allowed && risk != ActionRisk::Critical {
            format!(
                "Trust {:.2} grants {:?} level, action risk {:?} is within bounds",
                trust_value, effective_level, risk,
            )
        } else if risk == ActionRisk::Critical {
            "Critical actions always require approval".into()
        } else {
            format!(
                "Trust {:.2} grants {:?} level, but action requires {:?}",
                trust_value, effective_level, required_level,
            )
        };

        let decision = AutonomyDecision {
            allowed: allowed && risk != ActionRisk::Critical,
            requires_approval,
            autonomy_level: effective_level,
            trust_score: trust_value,
            action_risk: risk,
            reason,
        };

        self.decisions.write().push(decision.clone());
        decision
    }

    /// Record a successful action — earns trust in the domain
    pub fn record_success(&self, domain: &TrustDomain, risk: ActionRisk) {
        let earn_amount = match risk {
            ActionRisk::None => 0.01,
            ActionRisk::Low => 0.02,
            ActionRisk::Medium => 0.03,
            ActionRisk::High => 0.05,
            ActionRisk::Critical => 0.08,
        } * self.earn_rate;

        let mut scores = self.trust_scores.write();
        let score = scores
            .entry(domain.clone())
            .or_insert_with(|| TrustScore::new(domain.clone()));
        score.earn(
            earn_amount,
            &format!("Successful {:?} risk action", risk),
        );
    }

    /// Record a failed action — penalizes trust
    pub fn record_failure(&self, domain: &TrustDomain, risk: ActionRisk) {
        let penalty_amount = match risk {
            ActionRisk::None => 0.02,
            ActionRisk::Low => 0.05,
            ActionRisk::Medium => 0.10,
            ActionRisk::High => 0.20,
            ActionRisk::Critical => 0.30,
        } * self.penalty_rate;

        let mut scores = self.trust_scores.write();
        let score = scores
            .entry(domain.clone())
            .or_insert_with(|| TrustScore::new(domain.clone()));
        score.penalize(
            penalty_amount,
            &format!("Failed {:?} risk action", risk),
        );
    }

    /// Apply time-based decay to all trust scores
    pub fn apply_decay(&self) {
        for score in self.trust_scores.write().values_mut() {
            score.decay(self.decay_factor);
        }
    }

    /// Get trust score for a domain
    pub fn trust_score(&self, domain: &TrustDomain) -> Option<TrustScore> {
        self.trust_scores.read().get(domain).cloned()
    }

    /// Get current autonomy level for a domain
    pub fn autonomy_level(&self, domain: &TrustDomain) -> AutonomyLevel {
        self.trust_scores
            .read()
            .get(domain)
            .map(|s| s.autonomy_level().min(self.max_autonomy))
            .unwrap_or(AutonomyLevel::Observer)
    }

    /// Get all domain scores
    pub fn all_scores(&self) -> HashMap<TrustDomain, TrustScore> {
        self.trust_scores.read().clone()
    }

    /// Get decision history count
    pub fn decision_count(&self) -> usize {
        self.decisions.read().len()
    }
}

impl Default for GraduatedAutonomy {
    fn default() -> Self {
        Self::new(AutonomyLevel::Partner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_autonomy_is_observer() {
        let autonomy = GraduatedAutonomy::default();
        let level = autonomy.autonomy_level(&TrustDomain::global());
        assert_eq!(level, AutonomyLevel::Observer);
    }

    #[test]
    fn test_trust_earning() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Earn trust through many successful actions
        for _ in 0..30 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let score = autonomy.trust_score(&domain).unwrap();
        assert!(score.value > 0.5);
        assert!(score.autonomy_level() >= AutonomyLevel::Assistant);
    }

    #[test]
    fn test_trust_penalty() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Build up trust
        for _ in 0..20 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let before = autonomy.trust_score(&domain).unwrap().value;

        // Fail a high-risk action
        autonomy.record_failure(&domain, ActionRisk::High);
        let after = autonomy.trust_score(&domain).unwrap().value;

        assert!(after < before);
    }

    #[test]
    fn test_action_check_low_trust() {
        let autonomy = GraduatedAutonomy::default();
        let domain = TrustDomain::global();

        // Low trust should block high-risk actions
        let decision = autonomy.check_action(&domain, ActionRisk::High);
        assert!(!decision.allowed);
        assert!(decision.requires_approval);
    }

    #[test]
    fn test_autonomy_ceiling() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Assistant);
        let domain = TrustDomain::global();

        // Even with max trust, can't exceed ceiling
        for _ in 0..50 {
            autonomy.record_success(&domain, ActionRisk::High);
        }

        let level = autonomy.autonomy_level(&domain);
        assert!(level <= AutonomyLevel::Assistant);
    }

    #[test]
    fn test_trust_decay() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous)
            .with_decay_factor(0.9);
        let domain = TrustDomain::global();

        // Build trust
        for _ in 0..20 {
            autonomy.record_success(&domain, ActionRisk::Medium);
        }

        let before = autonomy.trust_score(&domain).unwrap().value;
        autonomy.apply_decay();
        let after = autonomy.trust_score(&domain).unwrap().value;

        assert!(after < before);
    }

    #[test]
    fn test_domain_specific_trust() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let file_domain = TrustDomain::new("file_operations");
        let net_domain = TrustDomain::new("network");

        // Build trust only in file domain
        for _ in 0..20 {
            autonomy.record_success(&file_domain, ActionRisk::Medium);
        }

        let file_level = autonomy.autonomy_level(&file_domain);
        let net_level = autonomy.autonomy_level(&net_domain);

        assert!(file_level > net_level);
    }

    #[test]
    fn test_critical_always_requires_approval() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        // Max out trust
        for _ in 0..100 {
            autonomy.record_success(&domain, ActionRisk::Critical);
        }

        let decision = autonomy.check_action(&domain, ActionRisk::Critical);
        assert!(decision.requires_approval);
    }

    #[test]
    fn test_trust_score_history() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_failure(&domain, ActionRisk::Medium);

        let score = autonomy.trust_score(&domain).unwrap();
        assert_eq!(score.history.len(), 2);
        assert!(score.history[0].delta > 0.0); // earn
        assert!(score.history[1].delta < 0.0); // penalize
    }

    #[test]
    fn test_autonomy_level_thresholds() {
        assert_eq!(AutonomyLevel::Observer.required_trust(), 0.0);
        assert_eq!(AutonomyLevel::Apprentice.required_trust(), 0.2);
        assert_eq!(AutonomyLevel::Assistant.required_trust(), 0.4);
        assert_eq!(AutonomyLevel::Partner.required_trust(), 0.7);
        assert_eq!(AutonomyLevel::Autonomous.required_trust(), 0.9);
    }

    #[test]
    fn test_success_rate_tracking() {
        let autonomy = GraduatedAutonomy::new(AutonomyLevel::Autonomous);
        let domain = TrustDomain::global();

        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_success(&domain, ActionRisk::Low);
        autonomy.record_failure(&domain, ActionRisk::Low);

        let score = autonomy.trust_score(&domain).unwrap();
        assert!((score.success_rate() - 2.0 / 3.0).abs() < 0.01);
    }
}
