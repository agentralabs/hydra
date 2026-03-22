//! THE FINAL LAYER 1 HARNESS
//! Run: cargo run -p hydra-reach --bin reach_test_harness

use hydra_horizon::{ActionExpansion, Horizon, PerceptionExpansion};
use hydra_reach::{
    apply_handoff, prepare_handoff, DeviceCapabilities, DeviceSession, OutputMode,
    ReachServer, SurfaceClass,
};
use hydra_skills::{SkillDomain, SkillManifest, SkillRegistry};

struct Test { name: &'static str, passed: bool, notes: Option<String> }
impl Test {
    fn pass(name: &'static str) -> Self { Self { name, passed: true, notes: None } }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self { name, passed: false, notes: Some(n.into()) }
    }
}

fn desktop_caps() -> DeviceCapabilities {
    DeviceCapabilities {
        has_keyboard: true, has_display: true, display_width: Some(1440),
        has_microphone: true, has_speaker: true, ..Default::default()
    }
}

fn glasses_caps() -> DeviceCapabilities {
    DeviceCapabilities {
        has_microphone: true, has_speaker: true, has_display: false,
        ..Default::default()
    }
}

fn mobile_caps() -> DeviceCapabilities {
    DeviceCapabilities {
        is_mobile: true, has_display: true, has_touch: true,
        has_microphone: true, display_width: Some(390), ..Default::default()
    }
}

fn test_horizon(tests: &mut Vec<Test>) {
    println!("\n-- hydra-horizon ------------------------------------");
    let h = Horizon::new();
    let ok = h.perception.value > 0.0 && h.action.value > 0.0;
    tests.push(if ok { Test::pass("Horizon: both start above zero") }
        else { Test::fail("Horizon: initial values", "zero") });

    let mut h = Horizon::new();
    let (pb, ab) = (h.perception.value, h.action.value);
    h.expand_perception(PerceptionExpansion::SisterConnected {
        sister_name: "memory".into() }).unwrap();
    h.expand_action(ActionExpansion::CapabilitySynthesized { name: "risk".into() });
    let ok = h.perception.value > pb && h.action.value > ab;
    tests.push(if ok { Test::pass("Horizon: both expand from events") }
        else { Test::fail("Horizon: expansion", "did not expand") });

    let before = h.perception.value;
    h.expand_perception(PerceptionExpansion::GenomeEntry { count_delta: 1000 }).unwrap();
    tests.push(if h.perception.value > before {
        Test::pass("Horizon: genome entries expand perception") }
        else { Test::fail("Horizon: genome expansion", "no change") });

    let expected = (h.perception.value * h.action.value).sqrt();
    tests.push(if (h.combined() - expected).abs() < 1e-10 {
        Test::pass("Horizon: combined = geometric mean") }
        else { Test::fail("Horizon: combined formula", "wrong") });

    let mut h2 = Horizon::new();
    for _ in 0..10_000 {
        h2.expand_action(ActionExpansion::CapabilitySynthesized { name: "x".into() });
    }
    tests.push(if h2.action.value <= 1.0 { Test::pass("Horizon: never exceeds 1.0") }
        else { Test::fail("Horizon: ceiling", format!("{:.4}", h2.action.value)) });
}

fn test_skills(tests: &mut Vec<Test>) {
    println!("\n-- hydra-skills -------------------------------------");
    let mut reg = SkillRegistry::new();
    let finance = SkillManifest::new("finance-v1", "Finance", "0.1.0", SkillDomain::Finance)
        .with_capabilities(vec!["risk.assess".into(), "portfolio.optimize".into()])
        .with_approaches(vec!["dcf".into()])
        .with_persona("finance-analyst");
    reg.load(finance).unwrap();
    tests.push(if reg.loaded_count() == 1 { Test::pass("Skills: finance loaded") }
        else { Test::fail("Skills: load", format!("{}", reg.loaded_count())) });

    reg.load(SkillManifest::new("sec-v1", "Security", "0.1.0", SkillDomain::Security)).unwrap();
    tests.push(Test::pass("Skills: two skills loaded"));

    reg.unload("finance-v1").unwrap();
    tests.push(if reg.loaded_count() == 1 { Test::pass("Skills: unload removes active") }
        else { Test::fail("Skills: unload", format!("{}", reg.loaded_count())) });
    tests.push(if reg.ever_loaded_count() == 2 {
        Test::pass("Skills: ever_loaded persists") }
        else { Test::fail("Skills: ever_loaded", format!("{}", reg.ever_loaded_count())) });

    let dup = SkillManifest::new("sec-v1", "Security", "0.1.0", SkillDomain::Security);
    tests.push(if reg.load(dup).is_err() { Test::pass("Skills: duplicate rejected") }
        else { Test::fail("Skills: duplicate", "allowed") });
}

fn test_reach(tests: &mut Vec<Test>) {
    println!("\n-- hydra-reach --------------------------------------");
    let (dc, gc, mc) = (desktop_caps(), glasses_caps(), mobile_caps());
    tests.push(if dc.infer_surface_class() == SurfaceClass::DesktopTui {
        Test::pass("Reach: desktop -> DesktopTui") }
        else { Test::fail("Reach: desktop", format!("{:?}", dc.infer_surface_class())) });
    tests.push(if gc.infer_surface_class() == SurfaceClass::WearableAudio {
        Test::pass("Reach: glasses -> WearableAudio") }
        else { Test::fail("Reach: glasses", format!("{:?}", gc.infer_surface_class())) });
    tests.push(if mc.infer_surface_class() == SurfaceClass::Mobile {
        Test::pass("Reach: mobile -> Mobile") }
        else { Test::fail("Reach: mobile", format!("{:?}", mc.infer_surface_class())) });

    let modes_ok = SurfaceClass::DesktopTui.preferred_output() == OutputMode::FullCockpit
        && SurfaceClass::WearableAudio.preferred_output() == OutputMode::VoiceOnly
        && SurfaceClass::Mobile.preferred_output() == OutputMode::CompanionView
        && SurfaceClass::ApiClient.preferred_output() == OutputMode::StructuredJson;
    tests.push(if modes_ok { Test::pass("Reach: output mode mapping correct") }
        else { Test::fail("Reach: output modes", "wrong") });

    let mut server = ReachServer::new(7474);
    let did = server.register_device("Mac", desktop_caps(), "token-001").unwrap();
    let gid = server.register_device("Glasses", glasses_caps(), "token-002").unwrap();
    tests.push(if server.device_count() == 2 { Test::pass("Reach: 2 devices registered") }
        else { Test::fail("Reach: devices", format!("{}", server.device_count())) });

    let _sid = server.connect(&did, "token-001").unwrap();
    tests.push(if server.active_session_count() == 1 {
        Test::pass("Reach: desktop session created") }
        else { Test::fail("Reach: session", "none") });
    tests.push(if server.connect(&gid, "wrong").is_err() {
        Test::pass("Reach: wrong token rejected") }
        else { Test::fail("Reach: auth", "accepted") });

    let mut gs = DeviceSession::new("glasses-001", OutputMode::VoiceOnly);
    gs.record_message("m1"); gs.record_message("m2"); gs.record_message("m3");
    let pkg = prepare_handoff(&gs, "desktop-001", Some("Phase 14".into()), 565);
    tests.push(if pkg.context_tail.len() == 3 { Test::pass("Reach: handoff context") }
        else { Test::fail("Reach: handoff", format!("{}", pkg.context_tail.len())) });
    let ds = apply_handoff(&pkg, OutputMode::FullCockpit).unwrap();
    let ok = ds.context_tail.len() == 3 && ds.output_mode == OutputMode::FullCockpit;
    tests.push(if ok { Test::pass("Reach: session continues on desktop") }
        else { Test::fail("Reach: continuity", "failed") });
    tests.push(if pkg.active_task.as_deref() == Some("Phase 14") {
        Test::pass("Reach: active task in handoff") }
        else { Test::fail("Reach: task", "lost") });
}

fn test_integration(tests: &mut Vec<Test>) {
    println!("\n-- integration --------------------------------------");
    let mut horizon = Horizon::new();
    let mut skills = SkillRegistry::new();
    let mut server = ReachServer::new(7474);

    let skill = SkillManifest::new("sec-v1", "Security", "0.1.0", SkillDomain::Security)
        .with_capabilities(vec!["threat".into(), "cve".into()]);
    let cc = skill.capabilities.len();
    skills.load(skill).unwrap();
    horizon.expand_action(ActionExpansion::SkillLoaded {
        skill_name: "sec-v1".into(), capability_count: cc });
    let gid = server.register_device("Glasses", glasses_caps(), "tok").unwrap();
    let _ = server.connect(&gid, "tok").unwrap();
    horizon.expand_perception(PerceptionExpansion::DeviceConnected {
        device_class: "WearableAudio".into() }).unwrap();

    let ok = skills.loaded_count() == 1 && server.active_session_count() == 1
        && horizon.action.value > 0.1 && horizon.perception.value > 0.1;
    tests.push(if ok { Test::pass("Integration: all systems coherent") }
        else { Test::fail("Integration", "incoherent") });
    println!("  P={:.4} A={:.4} C={:.4}", horizon.perception.value,
        horizon.action.value, horizon.combined());
}

fn main() {
    println!("===================================================");
    println!("  Phase 14 -- THE FINAL LAYER 1 PHASE");
    println!("===================================================");
    let mut tests = Vec::new();
    test_horizon(&mut tests);
    test_skills(&mut tests);
    test_reach(&mut tests);
    test_integration(&mut tests);

    println!();
    let (total, passed) = (tests.len(), tests.iter().filter(|t| t.passed).count());
    for t in &tests {
        if t.passed { println!("  PASS  {}", t.name); }
        else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes { println!("           {}", n); }
        }
    }
    println!("\n  Results: {}/{}", passed, total);
    if passed < total { std::process::exit(1); }
    else { println!("  LAYER 1 -- COMPLETE\n==================================================="); }
}
