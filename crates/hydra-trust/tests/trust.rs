//! Integration tests for hydra-trust.

use hydra_trust::{
    boltzmann_weight, spawn_decision, AgentState, TrustAgent, TrustField, TrustPhase, TrustScore,
    TrustTier,
};

#[test]
fn score_validation_rejects_out_of_range() {
    assert!(TrustScore::new(-0.01).is_err());
    assert!(TrustScore::new(1.01).is_err());
    assert!(TrustScore::new(f64::NAN).is_err());
    assert!(TrustScore::new(0.0).is_ok());
    assert!(TrustScore::new(1.0).is_ok());
}

#[test]
fn tier_energy_strictly_ordered() {
    assert!(TrustTier::Platinum.energy() < TrustTier::Gold.energy());
    assert!(TrustTier::Gold.energy() < TrustTier::Silver.energy());
    assert!(TrustTier::Silver.energy() < TrustTier::Bronze.energy());
}

#[test]
fn success_and_failure_tracking() {
    let mut agent = TrustAgent::new("tracker");
    agent.record_success();
    agent.record_success();
    assert_eq!(agent.total_successes, 2);
    assert_eq!(agent.consecutive_failures, 0);
    agent.record_failure("oops");
    assert_eq!(agent.total_failures, 1);
    assert_eq!(agent.consecutive_failures, 1);
    agent.record_success();
    assert_eq!(agent.consecutive_failures, 0);
}

#[test]
fn constitutional_violation_triggers_hold() {
    let mut agent = TrustAgent::new("violator");
    agent.record_constitutional_violation();
    assert_eq!(agent.state, AgentState::ConstitutionalHold);
    assert!(agent.constitutional_violation_ever);
}

#[test]
fn uniform_fleet_is_stable_or_elevated() {
    let mut field = TrustField::new();
    for i in 0..5 {
        field.add_agent(TrustAgent::new(format!("a{i}"))).unwrap();
    }
    let h = field.hamiltonian();
    // Default 0.5 → Elevated
    assert!(h.phase == TrustPhase::Elevated || h.phase == TrustPhase::Stable);
}

#[test]
fn field_violation_spike() {
    let mut field = TrustField::new();
    let agent = TrustAgent::new("v");
    let id = agent.id;
    field.add_agent(agent).unwrap();
    let result = field.record_constitutional_violation(&id, "bad action");
    assert!(result.is_err());
    let a = field.get_agent(&id).unwrap();
    assert!(a.is_on_hold());
}

#[test]
fn boltzmann_weight_ordering() {
    let t = 1.0;
    let w_plat = boltzmann_weight(TrustTier::Platinum, t);
    let w_gold = boltzmann_weight(TrustTier::Gold, t);
    let w_silver = boltzmann_weight(TrustTier::Silver, t);
    let w_bronze = boltzmann_weight(TrustTier::Bronze, t);
    assert!(w_plat > w_gold);
    assert!(w_gold > w_silver);
    assert!(w_silver > w_bronze);
}

#[test]
fn constitution_always_spawnable() {
    assert!(spawn_decision(TrustTier::Platinum, 0.001));
    assert!(spawn_decision(TrustTier::Platinum, 100.0));
}

#[test]
fn repeated_failures_quarantine() {
    let mut agent = TrustAgent::new("failing");
    for i in 0..5 {
        agent.record_failure(format!("fail {i}"));
    }
    assert!(agent.is_quarantined());
}
