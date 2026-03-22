//! Trust agent — an entity tracked by the trust field.

use crate::constants::*;
use crate::score::{TrustScore, TrustTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The operational state of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is operating normally.
    Active,
    /// Agent is quarantined due to repeated failures.
    Quarantined,
    /// Agent is on constitutional hold (violation detected).
    ConstitutionalHold,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Quarantined => write!(f, "Quarantined"),
            Self::ConstitutionalHold => write!(f, "ConstitutionalHold"),
        }
    }
}

/// An agent tracked by the trust thermodynamic field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAgent {
    /// Unique identifier for this agent.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Current trust score.
    pub score: TrustScore,
    /// Current operational state.
    pub state: AgentState,
    /// Number of consecutive failures.
    pub consecutive_failures: u32,
    /// Total successes recorded.
    pub total_successes: u64,
    /// Total failures recorded.
    pub total_failures: u64,
    /// Whether a constitutional violation has ever occurred.
    pub constitutional_violation_ever: bool,
    /// When this agent was created.
    pub created_at: DateTime<Utc>,
    /// Last state transition time.
    pub last_transition: DateTime<Utc>,
}

impl TrustAgent {
    /// Create a new agent with default trust score.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            score: TrustScore::default_score(),
            state: AgentState::Active,
            consecutive_failures: 0,
            total_successes: 0,
            total_failures: 0,
            constitutional_violation_ever: false,
            created_at: now,
            last_transition: now,
        }
    }

    /// Return the current trust tier.
    pub fn tier(&self) -> TrustTier {
        self.score.tier()
    }

    /// Record a successful operation.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.total_successes += 1;
        self.score.increase(TRUST_RECOVERY_RATE);
    }

    /// Record a failed operation.
    pub fn record_failure(&mut self, _reason: impl Into<String>) {
        self.consecutive_failures += 1;
        self.total_failures += 1;
        self.score.decrease(TRUST_PENALTY_RATE);
        self.check_auto_quarantine();
    }

    /// Record a constitutional violation. Always triggers hold.
    pub fn record_constitutional_violation(&mut self) {
        self.constitutional_violation_ever = true;
        self.state = AgentState::ConstitutionalHold;
        self.last_transition = Utc::now();
        self.score.decrease(CONSTITUTIONAL_VIOLATION_SPIKE);
    }

    /// Check if consecutive failures warrant auto-quarantine.
    fn check_auto_quarantine(&mut self) {
        if self.consecutive_failures >= QUARANTINE_FAILURE_THRESHOLD
            && self.state == AgentState::Active
        {
            self.state = AgentState::Quarantined;
            self.last_transition = Utc::now();
        }
    }

    /// Return true if the agent is active.
    pub fn is_active(&self) -> bool {
        self.state == AgentState::Active
    }

    /// Return true if the agent is quarantined.
    pub fn is_quarantined(&self) -> bool {
        self.state == AgentState::Quarantined
    }

    /// Return true if the agent is on constitutional hold.
    pub fn is_on_hold(&self) -> bool {
        self.state == AgentState::ConstitutionalHold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_agent_is_active() {
        let a = TrustAgent::new("test");
        assert!(a.is_active());
        assert_eq!(a.consecutive_failures, 0);
    }

    #[test]
    fn success_increases_score() {
        let mut a = TrustAgent::new("test");
        let before = a.score.value();
        a.record_success();
        assert!(a.score.value() > before);
    }

    #[test]
    fn failure_decreases_score() {
        let mut a = TrustAgent::new("test");
        let before = a.score.value();
        a.record_failure("oops");
        assert!(a.score.value() < before);
    }

    #[test]
    fn constitutional_violation_triggers_hold() {
        let mut a = TrustAgent::new("test");
        a.record_constitutional_violation();
        assert!(a.is_on_hold());
        assert!(a.constitutional_violation_ever);
    }

    #[test]
    fn repeated_failures_quarantine() {
        let mut a = TrustAgent::new("test");
        for i in 0..QUARANTINE_FAILURE_THRESHOLD {
            a.record_failure(format!("fail {i}"));
        }
        assert!(a.is_quarantined());
    }
}
