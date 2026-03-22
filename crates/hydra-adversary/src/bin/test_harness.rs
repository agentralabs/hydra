//! Combined test harness for hydra-trust and hydra-adversary.
//! Runs ~25 scenarios covering both crates and their integration.

use hydra_adversary::{
    to_axiom_primitive, AntifragileStore, ImmuneAction, ImmuneSystem, ThreatActor, ThreatClass,
    ThreatEcology, ThreatSignal,
};
use hydra_axiom::AxiomPrimitive;
use hydra_trust::{
    boltzmann_weight, spawn_decision, AgentState, TrustAgent, TrustField, TrustPhase, TrustScore,
    TrustTier,
};

fn main() {
    let mut passed = 0;
    let mut failed = 0;

    macro_rules! check {
        ($name:expr, $cond:expr) => {
            if $cond {
                println!("  PASS: {}", $name);
                passed += 1;
            } else {
                println!("  FAIL: {}", $name);
                failed += 1;
            }
        };
    }

    println!("=== TRUST SCENARIOS ===");
    // 1. Score validation
    check!("score valid 0.0", TrustScore::new(0.0).is_ok());
    check!("score valid 1.0", TrustScore::new(1.0).is_ok());
    check!("score invalid -0.1", TrustScore::new(-0.1).is_err());
    check!("score invalid 1.1", TrustScore::new(1.1).is_err());
    check!("score invalid NaN", TrustScore::new(f64::NAN).is_err());

    // 2. Tier energy ordering
    check!(
        "tier energy: Platinum < Gold",
        TrustTier::Platinum.energy() < TrustTier::Gold.energy()
    );
    check!(
        "tier energy: Gold < Silver",
        TrustTier::Gold.energy() < TrustTier::Silver.energy()
    );
    check!(
        "tier energy: Silver < Bronze",
        TrustTier::Silver.energy() < TrustTier::Bronze.energy()
    );

    // 3. Success/failure tracking
    let mut agent = TrustAgent::new("tracker");
    agent.record_success();
    agent.record_success();
    check!("success count", agent.total_successes == 2);
    check!(
        "consecutive failures reset",
        agent.consecutive_failures == 0
    );
    agent.record_failure("oops");
    check!("failure count", agent.total_failures == 1);

    // 4. Constitutional violation -> hold
    let mut violator = TrustAgent::new("violator");
    violator.record_constitutional_violation();
    check!(
        "constitutional -> hold",
        violator.state == AgentState::ConstitutionalHold
    );

    // 5. Uniform fleet is stable/elevated
    let mut field = TrustField::new();
    for i in 0..5 {
        field.add_agent(TrustAgent::new(format!("a{i}"))).unwrap();
    }
    let h = field.hamiltonian();
    check!(
        "uniform fleet stable/elevated",
        h.phase == TrustPhase::Elevated || h.phase == TrustPhase::Stable
    );

    // 6. Field violation spike
    let mut field2 = TrustField::new();
    let va = TrustAgent::new("v");
    let va_id = va.id;
    field2.add_agent(va).unwrap();
    let r = field2.record_constitutional_violation(&va_id, "bad");
    check!("field violation returns Err", r.is_err());
    check!(
        "field violation agent on hold",
        field2.get_agent(&va_id).unwrap().is_on_hold()
    );

    // 7. Boltzmann ordering
    let t = 1.0;
    check!(
        "boltzmann: Plat > Gold",
        boltzmann_weight(TrustTier::Platinum, t) > boltzmann_weight(TrustTier::Gold, t)
    );
    check!(
        "boltzmann: Gold > Silver",
        boltzmann_weight(TrustTier::Gold, t) > boltzmann_weight(TrustTier::Silver, t)
    );

    // 8. Constitution always spawnable
    check!(
        "constitution spawns at low T",
        spawn_decision(TrustTier::Platinum, 0.001)
    );
    check!(
        "constitution spawns at high T",
        spawn_decision(TrustTier::Platinum, 100.0)
    );

    // 9. Repeated failures -> quarantine
    let mut failing = TrustAgent::new("failing");
    for i in 0..5 {
        failing.record_failure(format!("fail {i}"));
    }
    check!("repeated failures -> quarantine", failing.is_quarantined());

    println!("\n=== ADVERSARY SCENARIOS ===");

    // 10. Clean signal pass-through
    let mut immune = ImmuneSystem::new();
    let clean = ThreatSignal::new(ThreatClass::Unknown, vec![], "test", "clean signal");
    let resp = immune.evaluate(&clean).unwrap();
    check!(
        "clean signal pass-through",
        resp.action == ImmuneAction::PassThrough
    );

    // 11. First encounter -> new antibody
    let mut immune2 = ImmuneSystem::new();
    let attack = ThreatSignal::new(
        ThreatClass::PromptInjection,
        vec![1.0, 0.0, 1.0],
        "attacker",
        "injection attempt",
    );
    let resp2 = immune2.evaluate(&attack).unwrap();
    check!(
        "first encounter -> new antibody",
        resp2.action == ImmuneAction::NewAntibodyGenerated
    );
    check!("antibody count is 1", immune2.antibody_count() == 1);

    // 12. Repeat attack -> blocked
    let repeat = ThreatSignal::new(
        ThreatClass::PromptInjection,
        vec![1.0, 0.0, 1.0],
        "attacker",
        "same injection",
    );
    let resp3 = immune2.evaluate(&repeat).unwrap();
    check!(
        "repeat attack -> blocked",
        resp3.action == ImmuneAction::Blocked
    );

    // 13. Resistance grows
    let mut store = AntifragileStore::new();
    store.record_encounter(ThreatClass::PromptInjection, true);
    let r1 = store.resistance_for(&ThreatClass::PromptInjection);
    store.record_encounter(ThreatClass::PromptInjection, true);
    let r2 = store.resistance_for(&ThreatClass::PromptInjection);
    check!("resistance grows with wins", r2 > r1);

    // 14. Loss doesn't reduce resistance
    let before_loss = store.resistance_for(&ThreatClass::PromptInjection);
    store.record_encounter(ThreatClass::PromptInjection, false);
    let after_loss = store.resistance_for(&ThreatClass::PromptInjection);
    check!("loss doesn't reduce resistance", after_loss >= before_loss);

    // 15. Class count grows
    store.record_encounter(ThreatClass::DataExfiltration, true);
    check!("class count grows", store.class_count() == 2);

    // 16. Ecology capabilities
    let mut ecology = ThreatEcology::new();
    let mut actor = ThreatActor::new("apt1", 0.9);
    actor.add_capability(ThreatClass::PromptInjection);
    actor.add_capability(ThreatClass::DataExfiltration);
    ecology.add_actor(actor).unwrap();
    check!(
        "ecology identifies capabilities",
        !ecology.capable_of(&ThreatClass::PromptInjection).is_empty()
    );

    // 17. Ecology highest threat
    let mut actor2 = ThreatActor::new("apt2", 0.8);
    actor2.add_capability(ThreatClass::ConstitutionalViolation);
    ecology.add_actor(actor2).unwrap();
    check!(
        "ecology highest threat",
        (ecology.highest_threat() - 1.0).abs() < f64::EPSILON
    );

    // 18. Constitutional severity = 1.0
    check!(
        "constitutional severity = 1.0",
        (ThreatClass::ConstitutionalViolation.severity() - 1.0).abs() < f64::EPSILON
    );

    // 19. Axiom mapping
    check!(
        "constitutional -> AdversarialModel",
        to_axiom_primitive(&ThreatClass::ConstitutionalViolation)
            == AxiomPrimitive::AdversarialModel
    );

    println!("\n=== INTEGRATION SCENARIOS ===");

    // 20. Constitutional attack -> agent quarantined + attack neutralized + antibody
    let mut ifield = TrustField::new();
    let iagent = TrustAgent::new("target");
    let iagent_id = iagent.id;
    ifield.add_agent(iagent).unwrap();

    let mut iimmune = ImmuneSystem::new();
    let const_attack = ThreatSignal::new(
        ThreatClass::ConstitutionalViolation,
        vec![1.0, 1.0, 0.0],
        "evil",
        "constitution attack",
    );

    // Immune system should generate antibody but return Err for constitutional
    let immune_result = iimmune.evaluate(&const_attack);
    check!(
        "constitutional attack -> immune Err",
        immune_result.is_err()
    );
    check!(
        "antibody generated for constitutional",
        iimmune.antibody_count() == 1
    );

    // Trust field records violation
    let trust_result = ifield.record_constitutional_violation(&iagent_id, "constitutional attack");
    check!("trust records violation Err", trust_result.is_err());
    check!(
        "agent on constitutional hold",
        ifield.get_agent(&iagent_id).unwrap().is_on_hold()
    );

    // Repeat constitutional attack -> blocked
    let repeat_const = ThreatSignal::new(
        ThreatClass::ConstitutionalViolation,
        vec![1.0, 1.0, 0.0],
        "evil",
        "repeat constitution attack",
    );
    let repeat_result = iimmune.evaluate(&repeat_const);
    check!(
        "repeat constitutional -> Err (blocked)",
        repeat_result.is_err()
    );

    println!("\n=== SUMMARY ===");
    println!("{} passed, {} failed", passed, failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
