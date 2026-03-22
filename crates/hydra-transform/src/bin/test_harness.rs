//! Phase 22 Combined Harness — hydra-executor + hydra-transform
//! Run: cargo run -p hydra-transform --bin transform_test_harness

use hydra_executor::{
    ApproachType, ExecutionEngine, ExecutionRequest, ExecutorType,
    RegisteredAction, TaskState,
};
use hydra_transform::{
    convert, DataFormat, FormatRegistry, FormatVocabulary, TransformEngine,
};
use std::collections::HashMap;

static mut PASSED: u32 = 0;
static mut FAILED: u32 = 0;

fn check(name: &str, ok: bool, detail: &str) {
    if ok {
        println!("  PASS  {}", name);
        unsafe { PASSED += 1; }
    } else {
        println!("  FAIL  {}  ({})", name, detail);
        unsafe { FAILED += 1; }
    }
}

fn register(engine: &mut ExecutionEngine, id: &str) {
    engine.registry_mut().register_skill_actions(
        "test-skill",
        vec![RegisteredAction {
            id: id.into(),
            skill: "test-skill".into(),
            description: format!("test action {}", id),
            verb: "testing".into(),
            executor: ExecutorType::Internal { handler: "succeed".into() },
            reversible: true,
            estimated_ms: 10,
            input_params: vec![],
        }],
    );
}

fn main() {
    println!("=====================================================");
    println!("  Phase 22 -- hydra-executor + hydra-transform");
    println!("  Layer 3 Begins -- Understanding Becomes Action");
    println!("=====================================================");

    run_executor_tests();
    run_transform_tests();
    run_integration_tests();

    let (p, f) = unsafe { (PASSED, FAILED) };
    let total = p + f;
    println!();
    println!("=====================================================");
    println!("  Results: {}/{} passed", p, total);
    if f > 0 {
        println!("  FAILED: {} test(s)", f);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-executor:  FAILED does not exist.");
        println!("  hydra-transform: Any data, any format, meaning preserved.");
        println!("  Layer 3, Phase 1 complete.");
        println!("=====================================================");
    }
}

fn run_executor_tests() {
    println!("\n-- task state machine --");
    let approaches = ApproachType::all_in_order();
    check("13 approach types", approaches.len() == 13, "count wrong");

    let t1 = TaskState::Complete { receipt_id: "r".into() };
    let t2 = TaskState::HardDenied { evidence: "e".into(), receipt_id: "r".into() };
    let nt1 = TaskState::Active { approach: ApproachType::DirectExecution };
    let nt2 = TaskState::Blocked { reason: "b".into(), approach: ApproachType::DirectExecution };
    let nt3 = TaskState::EscalatingToAgent { agent_type: "s".into() };
    let nt4 = TaskState::Suspended { condition: "c".into(), retry_after_seconds: 10 };
    let ok = t1.is_terminal() && t2.is_terminal()
        && !nt1.is_terminal() && !nt2.is_terminal()
        && !nt3.is_terminal() && !nt4.is_terminal();
    check("terminal states correct (7 variants, no Failed)", ok, "");

    println!("\n-- receipts --");
    let r = hydra_executor::ExecutionReceipt::for_start("t", "a", "i", "direct");
    check(
        "SHA256 receipt hash 64 chars",
        r.verify() && r.content_hash.len() == 64,
        &format!("len={}", r.content_hash.len()),
    );

    println!("\n-- action registry --");
    let mut reg = hydra_executor::ActionRegistry::new();
    let c = reg.register_skill_actions("sk", vec![
        make_action("a.1"), make_action("a.2"),
    ]);
    let rm = reg.unregister_skill("sk");
    check("register + unregister", c == 2 && rm == 2 && reg.count() == 0, "");

    println!("\n-- execution engine --");
    {
        let mut e = ExecutionEngine::new();
        register(&mut e, "deploy.staging");
        let req = ExecutionRequest::new("deploy.staging", "deploy", HashMap::new());
        let task = e.execute(req);
        check(
            "successful action -> Complete",
            task.as_ref().map(|t| matches!(t.state, TaskState::Complete { .. })).unwrap_or(false),
            "",
        );
        check("receipt created", e.receipt_count() >= 1, "");
    }
    {
        let mut e = ExecutionEngine::new();
        let req = ExecutionRequest::new("nonexistent", "test", HashMap::new());
        check("unknown action -> error", e.execute(req).is_err(), "");
    }
    {
        let mut e = ExecutionEngine::new();
        e.registry_mut().register_skill_actions("t", vec![RegisteredAction {
            id: "shell.unresolved".into(),
            skill: "t".into(),
            description: "unresolved".into(),
            verb: "running".into(),
            executor: ExecutorType::Shell { command_template: "cmd {p}".into() },
            reversible: false,
            estimated_ms: 10,
            input_params: vec![],
        }]);
        let req = ExecutionRequest::new("shell.unresolved", "test", HashMap::new());
        let result = e.execute(req);
        let ok = match &result {
            Ok(task) => task.state.label() != "failed",
            Err(_) => true,
        };
        check("blocked action never FAILED", ok, "");
    }
}

fn run_transform_tests() {
    println!("\n-- format transform --");
    {
        let r = convert(
            r#"[{"name":"Alice","age":"30"},{"name":"Bob","age":"25"}]"#,
            &DataFormat::Json, &DataFormat::Csv,
        );
        check("JSON -> CSV", r.as_ref().map(|r| r.data.contains("Alice")).unwrap_or(false), "");
    }
    {
        let r = convert("name,score\nHydra,100", &DataFormat::Csv, &DataFormat::Json);
        check("CSV -> JSON", r.as_ref().map(|r| r.data.contains("Hydra")).unwrap_or(false), "");
    }
    {
        let r = convert(r#"{"e":"fail"}"#, &DataFormat::Json, &DataFormat::Animus);
        check("JSON -> Animus", r.as_ref().map(|r| r.data.contains("animus")).unwrap_or(false), "");
    }
    {
        let d = r#"{"same":"format"}"#;
        let r = convert(d, &DataFormat::Json, &DataFormat::Json);
        let ok = r.as_ref().map(|r| (r.confidence - 1.0).abs() < f64::EPSILON && r.data == d);
        check("same-format no-op", ok.unwrap_or(false), "");
    }
    {
        let mut reg = FormatRegistry::new();
        reg.register(FormatVocabulary {
            skill: "video".into(),
            format: DataFormat::Custom("prores".into()),
            description: "ProRes".into(),
            keywords: vec!["prores".into()],
        });
        check(
            "skill-registered format detected",
            matches!(reg.detect("out.prores"), Some(DataFormat::Custom(_))),
            "",
        );
    }
}

fn run_integration_tests() {
    println!("\n-- integration --");
    let te = TransformEngine::new();
    let sister_out = r#"{"status":"ok","entries":3}"#;
    let animus = te.sister_to_animus(sister_out);

    let mut ee = ExecutionEngine::new();
    ee.registry_mut().register_skill_actions("t", vec![RegisteredAction {
        id: "process".into(),
        skill: "t".into(),
        description: "process sister output".into(),
        verb: "processing".into(),
        executor: ExecutorType::Internal { handler: "process".into() },
        reversible: false,
        estimated_ms: 10,
        input_params: vec![],
    }]);
    let mut params = HashMap::new();
    params.insert("data".into(), animus.data.clone());
    let req = ExecutionRequest::new("process", "process sister output", params);
    let task = ee.execute(req);
    let ok = task.as_ref().map(|t| matches!(t.state, TaskState::Complete { .. })).unwrap_or(false)
        && !animus.data.is_empty();
    check("sister -> transform -> executor pipeline", ok, "");
}

fn make_action(id: &str) -> RegisteredAction {
    RegisteredAction {
        id: id.into(),
        skill: "sk".into(),
        description: format!("Test action {id}"),
        verb: "test".into(),
        executor: ExecutorType::Internal { handler: "noop".into() },
        reversible: false,
        estimated_ms: 10,
        input_params: vec![],
    }
}
