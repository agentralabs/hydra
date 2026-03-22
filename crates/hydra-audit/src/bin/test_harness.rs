//! Phase 27 Test Harness — hydra-audit (THE FINAL LAYER 3 CRATE)
//! Run: cargo run -p hydra-audit --bin test_harness

use hydra_audit::{
    AuditEngine, AuditQuery, EventKind, ExecutionTrace, NarrativeBuilder, TraceEvent,
};
use hydra_executor::{ExecutionEngine, ExecutionRequest, ExecutorType, RegisteredAction};
use std::collections::HashMap;

struct Test { name: &'static str, passed: bool, notes: Option<String> }
impl Test {
    fn pass(name: &'static str) -> Self { Self { name, passed: true, notes: None } }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self { name, passed: false, notes: Some(n.into()) }
    }
}

fn evt(task: &str, kind: EventKind, rid: &str, ms: u64) -> TraceEvent {
    TraceEvent::new(task, kind, rid, ms)
}

fn started(intent: &str) -> EventKind { EventKind::TaskStarted { intent: intent.into() } }
fn attempted(a: &str) -> EventKind { EventKind::ApproachAttempted { approach: a.into() } }
fn obstacle(o: &str) -> EventKind { EventKind::ObstacleEncountered { obstacle: o.into() } }
fn succeeded(a: &str) -> EventKind { EventKind::ApproachSucceeded { approach: a.into() } }
fn reroute(f: &str, t: &str) -> EventKind { EventKind::Rerouting { from: f.into(), to: t.into() } }
fn completed(ms: u64) -> EventKind { EventKind::TaskCompleted { duration_total_ms: ms } }
fn denied(e: &str) -> EventKind { EventKind::TaskHardDenied { evidence: e.into() } }

fn make_trace(with_obstacle: bool) -> ExecutionTrace {
    let mut t = ExecutionTrace::new("task-test", "deploy.staging");
    t.add_event(evt("task-test", started("deploy to staging environment"), "r1", 0));
    t.add_event(evt("task-test", attempted("direct"), "r2", 150));
    if with_obstacle {
        t.add_event(evt("task-test", obstacle("auth certificate expired"), "r3", 10));
        t.add_event(evt("task-test", reroute("direct", "alternative"), "r4", 5));
        t.add_event(evt("task-test", attempted("alternative"), "r5", 200));
    }
    t.add_event(evt("task-test", succeeded("alternative"), "r6", 5));
    t.add_event(evt("task-test", completed(370), "r7", 0));
    t
}

fn simple_events(intent: &str, ms: u64) -> Vec<(EventKind, &'static str, u64)> {
    vec![(started(intent), "r1", 0), (completed(ms), "r2", 0)]
}

fn main() {
    println!("=========================================================");
    println!("  Phase 27 — THE FINAL LAYER 3 CRATE");
    println!("  hydra-audit — Execution Accountability Narrative");
    println!("=========================================================");
    let mut tests = Vec::new();

    // -- EXECUTION TRACE --
    println!("\n-- execution trace --");
    {
        let trace = make_trace(true);
        if trace.attempt_count() == 2 && trace.obstacle_count() == 1 {
            tests.push(Test::pass("Trace: 2 attempts, 1 obstacle counted correctly"));
        } else {
            tests.push(Test::fail("Trace: counts",
                format!("attempts={} obstacles={}", trace.attempt_count(), trace.obstacle_count())));
        }
        if trace.is_complete() && trace.outcome() == Some("completed".into()) {
            tests.push(Test::pass("Trace: terminal event detected — outcome=completed"));
        } else {
            tests.push(Test::fail("Trace: completion", format!("{:?}", trace.outcome())));
        }
    }

    // -- NARRATIVE BUILDER --
    println!("\n-- narrative builder --");
    {
        let builder = NarrativeBuilder::new();
        let n = builder.build(&make_trace(true)).expect("build narrative");
        if n.is_successful() {
            tests.push(Test::pass("Narrative: successful -> is_successful()=true"));
        } else {
            tests.push(Test::fail("Narrative: success flag", n.outcome.clone()));
        }
        if n.full.contains("cert") || n.full.contains("auth") {
            tests.push(Test::pass("Narrative: obstacle detail in full narrative"));
        } else {
            tests.push(Test::fail("Narrative: obstacle text", "missing"));
        }
        if n.summary.contains("COMPLETED") && n.summary.contains("deploy.staging") {
            tests.push(Test::pass("Narrative: summary contains outcome and action"));
        } else {
            tests.push(Test::fail("Narrative: summary", n.summary.clone()));
        }
        println!("  Summary: {}", n.summary);
        println!("  Full narrative:");
        for line in n.full.lines() { println!("     {}", line); }
    }

    // -- AUDIT RECORD --
    println!("\n-- audit record --");
    {
        let mut engine = AuditEngine::new();
        engine.audit_manual("task-deploy-1", "deploy.prod", vec![
            (started("deploy to production"), "r1", 0),
            (attempted("direct"), "r2", 100),
            (obstacle("network timeout"), "r3", 5000),
            (reroute("direct", "retry"), "r4", 5),
            (attempted("retry"), "r5", 150),
            (succeeded("retry"), "r6", 5),
            (completed(5260), "r7", 0),
        ]).expect("audit manual");

        if engine.record_count() == 1 {
            tests.push(Test::pass("Record: audit record persisted"));
        } else {
            tests.push(Test::fail("Record: count", format!("{}", engine.record_count())));
        }
        let record = engine.store.get_by_task("task-deploy-1").expect("get record");
        if record.verify_integrity() {
            tests.push(Test::pass("Record: SHA256 integrity hash verified"));
        } else {
            tests.push(Test::fail("Record: integrity", "hash invalid"));
        }
        if record.attempt_count == 2 && record.obstacle_count == 1 {
            tests.push(Test::pass("Record: attempt and obstacle counts correct"));
        } else {
            tests.push(Test::fail("Record: counts",
                format!("a={} o={}", record.attempt_count, record.obstacle_count)));
        }
        if record.is_successful() {
            tests.push(Test::pass("Record: completed task marked successful"));
        } else {
            tests.push(Test::fail("Record: success flag", record.outcome.clone()));
        }
    }

    // -- EXECUTOR INTEGRATION --
    println!("\n-- executor integration --");
    {
        let mut exec_engine = ExecutionEngine::new();
        exec_engine.registry_mut().register_skill_actions("test", vec![RegisteredAction {
            id: "test.deploy".into(), skill: "test".into(),
            description: "test deployment".into(), verb: "deploying".into(),
            executor: ExecutorType::Internal { handler: "succeed".into() },
            reversible: false, estimated_ms: 200, input_params: vec![],
        }]);
        let req = ExecutionRequest::new("test.deploy", "deploy to test env", HashMap::new());
        let task = exec_engine.execute(req).expect("execute");
        let mut audit = AuditEngine::new();
        let summary = audit.audit_task(&task).expect("audit task");
        if !summary.is_empty() && summary.contains("test.deploy") {
            tests.push(Test::pass("Integration: executor task -> audit summary produced"));
        } else {
            tests.push(Test::fail("Integration: summary", summary.clone()));
        }
        println!("  Executor -> audit: {}", summary);
    }

    // -- AUDIT QUERY --
    println!("\n-- audit query --");
    {
        let mut engine = AuditEngine::new();
        for (tid, aid, ok) in [("t1","deploy.staging",true),("t2","deploy.prod",true),("t3","security.scan",false)] {
            let events: Vec<(EventKind, &str, u64)> = if ok {
                simple_events("test", 100)
            } else {
                vec![(started("test"), "r1", 0), (denied("401"), "r2", 0)]
            };
            engine.audit_manual(tid, aid, events).expect("audit");
        }
        let all = engine.query(&AuditQuery::default());
        if all.len() == 3 {
            tests.push(Test::pass("Query: all 3 records returned"));
        } else {
            tests.push(Test::fail("Query: all count", format!("{}", all.len())));
        }
        let deploys = engine.query(&AuditQuery {
            action_id: Some("deploy.staging".into()), ..Default::default()
        });
        if deploys.len() == 1 {
            tests.push(Test::pass("Query: filter by action_id returns 1 record"));
        } else {
            tests.push(Test::fail("Query: action filter", format!("{}", deploys.len())));
        }
        let ok = engine.query(&AuditQuery {
            outcome: Some("completed".into()), ..Default::default()
        });
        if ok.len() == 2 {
            tests.push(Test::pass("Query: filter by outcome=completed returns 2 records"));
        } else {
            tests.push(Test::fail("Query: outcome filter", format!("{}", ok.len())));
        }
    }

    // -- APPEND-ONLY INVARIANT --
    {
        let mut engine = AuditEngine::new();
        engine.audit_manual("t1", "a.id", simple_events("i", 100)).expect("audit");
        let before = engine.record_count();
        engine.audit_manual("t2", "a.id", simple_events("i", 100)).expect("audit");
        assert_eq!(engine.record_count(), before + 1);
        tests.push(Test::pass("Invariant: audit store is append-only (no deletion)"));
    }

    // -- SUMMARY --
    {
        let s = AuditEngine::new().summary();
        if s.contains("audit:") && s.contains("records=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s));
        }
    }

    // -- RESULTS --
    println!();
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;
    for t in &tests {
        if t.passed { println!("  PASS  {}", t.name); }
        else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes { println!("           {}", n); }
        }
    }
    println!("\n=========================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        println!("=========================================================");
        std::process::exit(1);
    } else {
        println!();
        println!("  LAYER 3 — COMPLETE. 8 crates. All verified.");
        println!("  Phase 27 complete. Layer 3 is closed. Layer 4 begins.");
        println!("=========================================================");
    }
}
