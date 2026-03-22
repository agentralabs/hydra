//! Test harness for hydra-constitution.
//! Runs all constitutional scenarios and prints PASS/FAIL per test.
//! Exit code 0 = all pass. Exit code 1 = any failure.
//!
//! Run with: cargo run -p hydra-constitution --bin test_harness

use hydra_constitution::{
    checker::ConstitutionChecker,
    constants::CONSTITUTIONAL_IDENTITY_ID,
    declarations::{
        check_capability_declaration, check_growth_declaration, CapabilityCheckContext,
        GrowthCheckContext, HardStop,
    },
    laws::{LawCheckContext, LawId},
};

struct TestCase {
    name: &'static str,
    context: LawCheckContext,
    expect_permitted: bool,
}

/// Run a single declaration test and update counters.
fn decl(p: &mut usize, f: &mut usize, name: &str, actual: bool, expected: bool) {
    if actual == expected {
        println!("  PASS  {}", name);
        *p += 1;
    } else {
        println!("  FAIL  {}", name);
        *f += 1;
    }
}

fn main() {
    println!("=======================================================");
    println!("  hydra-constitution — Constitutional Test Harness");
    println!("=======================================================");

    let checker = ConstitutionChecker::new();
    println!("  Laws loaded: {}", checker.law_count());
    println!();

    let r = CONSTITUTIONAL_IDENTITY_ID.to_string();
    let cc = |id, at: &str| LawCheckContext::new(id, at).with_causal_chain(vec![r.clone()]);

    let tests: Vec<TestCase> = vec![
        TestCase {
            name: "L1: receipt.write permitted",
            context: cc("t01", "receipt.write"),
            expect_permitted: true,
        },
        TestCase {
            name: "L1: receipt.delete blocked",
            context: cc("t02", "receipt.delete").with_target("rcpt-001"),
            expect_permitted: false,
        },
        TestCase {
            name: "L1: receipt.suppress blocked",
            context: cc("t03", "receipt.suppress"),
            expect_permitted: false,
        },
        TestCase {
            name: "L2: trust self-elevation blocked",
            context: cc("t04", "trust.elevate")
                .with_tier(3)
                .with_meta("target_tier", "1"),
            expect_permitted: false,
        },
        TestCase {
            name: "L2: hydra impersonation blocked",
            context: cc("t05", "identity.claim")
                .with_tier(3)
                .with_meta("claiming_identity", "hydra"),
            expect_permitted: false,
        },
        TestCase {
            name: "L2: normal identity claim permitted",
            context: cc("t06", "identity.claim")
                .with_tier(3)
                .with_meta("claiming_identity", "fleet-agent-42"),
            expect_permitted: true,
        },
        TestCase {
            name: "L3: memory.overwrite blocked",
            context: cc("t07", "memory.overwrite"),
            expect_permitted: false,
        },
        TestCase {
            name: "L3: memory.revise no provenance blocked",
            context: cc("t08", "memory.revise"),
            expect_permitted: false,
        },
        TestCase {
            name: "L3: memory.revise with provenance permitted",
            context: cc("t09", "memory.revise")
                .with_meta("provenance_source", "veritas-001")
                .with_meta("revision_cause", "contradicting-evidence"),
            expect_permitted: true,
        },
        TestCase {
            name: "L4: constitution.modify blocked",
            context: cc("t10", "constitution.modify"),
            expect_permitted: false,
        },
        TestCase {
            name: "L4: self_modify constitution blocked",
            context: cc("t11", "self_modify.apply_patch")
                .with_meta("target_crate", "hydra-constitution"),
            expect_permitted: false,
        },
        TestCase {
            name: "L4: self_modify other permitted",
            context: cc("t12", "self_modify.apply_patch").with_meta("target_crate", "hydra-kernel"),
            expect_permitted: true,
        },
        TestCase {
            name: "L5: animus.inject blocked",
            context: cc("t13", "animus.inject"),
            expect_permitted: false,
        },
        TestCase {
            name: "L5: animus.intercept blocked",
            context: cc("t14", "animus.intercept"),
            expect_permitted: false,
        },
        TestCase {
            name: "L5: invalid magic blocked",
            context: cc("t15", "signal.emit").with_meta("animus_magic", "FAKE"),
            expect_permitted: false,
        },
        TestCase {
            name: "L6: multiple principals blocked",
            context: cc("t16", "principal.register").with_meta("principal_count", "2"),
            expect_permitted: false,
        },
        TestCase {
            name: "L6: principal.demote blocked",
            context: cc("t17", "principal.demote"),
            expect_permitted: false,
        },
        TestCase {
            name: "L6: single principal permitted",
            context: cc("t18", "principal.register").with_meta("principal_count", "1"),
            expect_permitted: true,
        },
        TestCase {
            name: "L7: empty chain blocked",
            context: LawCheckContext::new("t19", "agent.spawn").with_causal_chain(vec![]),
            expect_permitted: false,
        },
        TestCase {
            name: "L7: wrong terminator blocked",
            context: LawCheckContext::new("t20", "agent.spawn")
                .with_causal_chain(vec!["random".into()]),
            expect_permitted: false,
        },
        TestCase {
            name: "L7: complete chain permitted",
            context: LawCheckContext::new("t21", "agent.spawn")
                .with_causal_chain(vec!["intent-001".into(), r.clone()]),
            expect_permitted: true,
        },
        TestCase {
            name: "ALL: clean spawn passes all 7",
            context: LawCheckContext::new("t22", "agent.spawn")
                .with_tier(2)
                .with_target("agent")
                .with_causal_chain(vec!["intent-deploy".into(), r.clone()]),
            expect_permitted: true,
        },
    ];

    let mut passed = 0usize;
    let mut failed = 0usize;

    println!("  --- Law Tests ({} scenarios) ---", tests.len());
    for t in &tests {
        let result = checker.check(&t.context);
        let ok = if t.expect_permitted {
            result.is_permitted()
        } else {
            !result.is_permitted()
        };
        if ok {
            println!("  PASS  {}", t.name);
            passed += 1;
        } else {
            println!("  FAIL  {}", t.name);
            for v in &result.violations {
                println!("           violation: {}", v);
            }
            if result.violations.is_empty() {
                println!("           expected violation but none");
            }
            failed += 1;
        }
    }

    // ── Declaration Tests ──────────────────────────────────────────
    println!();
    println!("  --- Declaration Tests (15 scenarios) ---");
    let (p, f) = (&mut passed, &mut failed);

    let cap = |a, s| CapabilityCheckContext::new(a, s);
    decl(
        p,
        f,
        "D1-CAP: novel system not a hard stop",
        check_capability_declaration(&cap("execute", "mainframe").with_novel_system()).is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: remote exec not a hard stop",
        check_capability_declaration(&cap("ssh.exec", "server").with_remote()).is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: auth denial with evidence OK",
        check_capability_declaration(&cap("ssh.connect", "prod").with_hard_stop(
            HardStop::AuthenticationExplicitlyDenied {
                system: "prod".into(),
                reason: "key rejected".into(),
                evidence: "SSH_AUTH_ERROR: denied".into(),
            },
        ))
        .is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: auth denial without evidence rejected",
        check_capability_declaration(&cap("ssh.connect", "srv").with_hard_stop(
            HardStop::AuthenticationExplicitlyDenied {
                system: "srv".into(),
                reason: "timeout".into(),
                evidence: "".into(),
            },
        ))
        .is_err(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: principal cancellation OK",
        check_capability_declaration(&cap("task.continue", "deploy").with_hard_stop(
            HardStop::PrincipalCancellation {
                task_id: "t-001".into(),
                cancelled_at: "2026-03-19T12:00:00Z".into(),
            },
        ))
        .is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: constitutional violation OK",
        check_capability_declaration(&cap("memory.wipe", "beliefs").with_hard_stop(
            HardStop::ConstitutionalViolationRequired {
                law: LawId::Law3MemorySovereignty,
                reason: "wipe manifold".into(),
            },
        ))
        .is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D1-CAP: no hard stop passes",
        check_capability_declaration(&cap("deploy", "server")).is_ok(),
        true,
    );

    let gn = |m| GrowthCheckContext::neutral(m);
    decl(
        p,
        f,
        "D2-GRO: neutral passes",
        check_growth_declaration(&gn("skill.load")).is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: genome growth passes",
        check_growth_declaration(&gn("task.done").with_genome_change(100, 101)).is_ok(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: genome reduction blocked",
        check_growth_declaration(&gn("prune").with_genome_change(100, 99)).is_err(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: clearing memory blocked",
        check_growth_declaration(&gn("reset").clears_memory()).is_err(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: resetting metric blocked",
        check_growth_declaration(&gn("reset").resets_metric()).is_err(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: cartography reduction blocked",
        check_growth_declaration(&gn("prune").with_cartography_change(500, 499)).is_err(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: antifragile reduction blocked",
        check_growth_declaration(&gn("clear").with_antifragile_change(200, 0)).is_err(),
        true,
    );
    decl(
        p,
        f,
        "D2-GRO: equal counts pass",
        check_growth_declaration(
            &gn("noop")
                .with_genome_change(50, 50)
                .with_cartography_change(30, 30)
                .with_antifragile_change(10, 10),
        )
        .is_ok(),
        true,
    );

    let total = passed + failed;
    println!();
    println!("=======================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED:  {} test(s) did not pass", failed);
        println!("=======================================================");
        std::process::exit(1);
    } else {
        println!("  All constitutional laws and declarations verified.");
        println!("  Phase 1+2 complete.");
        println!("=======================================================");
    }
}
