//! Phase 23 Combined Harness — hydra-environment + hydra-protocol
//! Run: cargo run -p hydra-protocol --bin test_harness

use hydra_environment::{
    check_requirements, EnvironmentClass, EnvironmentEngine, RequiredBinary, SkillRequirements,
};
use hydra_protocol::{
    adapt_to_protocol, infer_from_target, ConnectionLifecycle, ProtocolEngine, ProtocolFamily,
};

struct Test {
    name: &'static str,
    passed: bool,
    notes: Option<String>,
}

impl Test {
    fn pass(name: &'static str) -> Self {
        Self {
            name,
            passed: true,
            notes: None,
        }
    }
    fn fail(name: &'static str, n: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            notes: Some(n.into()),
        }
    }
}

fn main() {
    println!("===================================================");
    println!("  Phase 23 — hydra-environment + hydra-protocol");
    println!("  Layer 3, Phase 2: Any Environment. Any Protocol.");
    println!("===================================================");

    let mut tests = Vec::new();

    // -- ENVIRONMENT ------------------------------------------------
    println!("\n-- hydra-environment ------------------------------------");

    {
        let mut engine = EnvironmentEngine::new();
        let profile = engine.probe().expect("probe failed");
        if profile.capabilities.cpu_cores > 0 && profile.capabilities.ram_mb > 0 {
            tests.push(Test::pass("Env: probe detects CPU cores and RAM"));
        } else {
            tests.push(Test::fail(
                "Env: probe",
                format!(
                    "cores={} ram={}",
                    profile.capabilities.cpu_cores, profile.capabilities.ram_mb
                ),
            ));
        }
        println!("  i  {}", engine.summary());
    }

    {
        let mut engine = EnvironmentEngine::new();
        let reqs = SkillRequirements::new("hello-skill");
        engine.register_requirements(reqs);
        engine.probe().expect("probe failed");
        let outcome = engine.check_skill("hello-skill").expect("check failed");
        if outcome.can_execute() {
            tests.push(Test::pass("Env: minimal skill requirements -> can execute"));
        } else {
            tests.push(Test::fail("Env: minimal skill", "blocked"));
        }
    }

    {
        use hydra_environment::profile::{EnvironmentCapabilities, EnvironmentProfile, OsType};
        let profile_no_binary = EnvironmentProfile::new(
            EnvironmentClass::LocalMachine,
            EnvironmentCapabilities {
                ram_mb: 8192,
                disk_mb: 100_000,
                cpu_cores: 4,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::detect(),
            },
        );

        let mut reqs = SkillRequirements::new("video-editor");
        reqs.binaries.push(RequiredBinary {
            name: "nonexistent-binary-xyz".into(),
            required: true,
            install_hint: "cannot install".into(),
            fallback: None,
        });

        let outcome = check_requirements(&reqs, &profile_no_binary);
        if !outcome.can_execute() {
            tests.push(Test::pass("Env: missing required binary -> blocked"));
        } else {
            tests.push(Test::fail("Env: binary blocking", "not blocked"));
        }
    }

    {
        use hydra_environment::profile::{EnvironmentCapabilities, EnvironmentProfile, OsType};
        let low_ram_profile = EnvironmentProfile::new(
            EnvironmentClass::LocalMachine,
            EnvironmentCapabilities {
                ram_mb: 256,
                disk_mb: 50_000,
                cpu_cores: 2,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::detect(),
            },
        );

        let mut reqs = SkillRequirements::new("test-skill");
        reqs.min_ram_mb = 1024;
        reqs.low_resource_threshold_mb = 128;

        let outcome = check_requirements(&reqs, &low_ram_profile);
        if outcome.can_execute() {
            tests.push(Test::pass(
                "Env: low RAM but above threshold -> degraded (still runs)",
            ));
        } else {
            tests.push(Test::fail(
                "Env: degraded mode",
                outcome.label().to_string(),
            ));
        }
    }

    {
        let mut engine = EnvironmentEngine::new();
        let r = engine.check_skill("ghost-skill");
        if r.is_err() {
            tests.push(Test::pass(
                "Env: unknown skill -> RequirementsNotRegistered error",
            ));
        } else {
            tests.push(Test::fail("Env: unknown skill error", "no error"));
        }
    }

    // -- PROTOCOL ---------------------------------------------------
    println!("\n-- hydra-protocol ---------------------------------------");

    {
        let cases = vec![
            ("https://api.example.com/data", ProtocolFamily::RestHttp),
            ("https://api.example.com/graphql", ProtocolFamily::GraphQL),
            ("wss://stream.example.com/events", ProtocolFamily::WebSocket),
            ("kafka-broker.internal:9092", ProtocolFamily::Kafka),
            (
                "mainframe.corp.internal/jcl/batch",
                ProtocolFamily::CobolJcl,
            ),
        ];
        let mut all_correct = true;
        for (target, expected) in &cases {
            let hint = infer_from_target(target);
            if &hint.likely_family != expected {
                all_correct = false;
                tests.push(Test::fail(
                    "Protocol: inference",
                    format!(
                        "'{}' -> {:?} expected {:?}",
                        target, hint.likely_family, expected
                    ),
                ));
            }
        }
        if all_correct {
            tests.push(Test::pass(
                "Protocol: 5/5 targets correctly inferred (REST, GraphQL, WS, Kafka, COBOL)",
            ));
        }
    }

    {
        let families = vec![
            ProtocolFamily::RestHttp,
            ProtocolFamily::GraphQL,
            ProtocolFamily::Grpc,
            ProtocolFamily::Mqtt,
            ProtocolFamily::CobolJcl,
        ];
        let mut all_receipted = true;
        for family in &families {
            let r = adapt_to_protocol("target", "intent", None, family).expect("adaptation failed");
            if r.request.receipt_id.is_empty() {
                all_receipted = false;
            }
        }
        if all_receipted {
            tests.push(Test::pass(
                "Protocol: every adaptation receipted (constitutional -- all 5 families)",
            ));
        } else {
            tests.push(Test::fail("Protocol: receipts", "some missing"));
        }
    }

    {
        let r = adapt_to_protocol(
            "mainframe.corp:23",
            "BATCH_MIGRATE_COBOL",
            Some("//SRCPGM EXEC PGM=MIGRATE"),
            &ProtocolFamily::CobolJcl,
        )
        .expect("adaptation failed");
        let body = r.request.body.as_deref().unwrap_or("");
        if body.contains("HYDRAJOB") && body.contains("BATCH_MIGRATE_COBOL") {
            tests.push(Test::pass(
                "Protocol: COBOL/JCL adaptation -- JCL format correct",
            ));
            println!("  i  COBOL/JCL adaptation:");
            println!("     method: {}", r.request.method);
            let preview_len = 50.min(body.len());
            println!("     body preview: {}...", &body[..preview_len]);
        } else {
            tests.push(Test::fail("Protocol: COBOL/JCL", "JCL format wrong"));
        }
    }

    {
        let mut lc = ConnectionLifecycle::new("https://api.example.com", ProtocolFamily::RestHttp);
        lc.connect().expect("connect failed");
        if lc.is_connected() {
            tests.push(Test::pass(
                "Protocol: connection lifecycle -- connect/disconnect",
            ));
            lc.disconnect();
            assert!(!lc.is_connected());
        } else {
            tests.push(Test::fail("Protocol: lifecycle", "not connected"));
        }
    }

    {
        let mut engine = ProtocolEngine::new();
        let r = engine
            .send(
                "https://api.example.com/deploy",
                "POST /deploy",
                Some(r#"{"env":"staging"}"#),
            )
            .expect("send failed");
        if r.success && !r.receipt_id.is_empty() {
            tests.push(Test::pass("Protocol: engine send -> success + receipt"));
        } else {
            tests.push(Test::fail(
                "Protocol: engine send",
                format!("success={}", r.success),
            ));
        }
    }

    // -- INTEGRATION ------------------------------------------------
    println!("\n-- integration: environment + protocol together --------");

    {
        let mut env = EnvironmentEngine::new();
        env.probe().expect("probe failed");

        let mut reqs = SkillRequirements::new("api-skill");
        reqs.min_ram_mb = 256;
        env.register_requirements(reqs);

        let env_outcome = env.check_skill("api-skill").expect("check failed");

        let mut proto = ProtocolEngine::new();
        let proto_result = proto
            .send("https://api.example.com/status", "GET /status", None)
            .expect("send failed");

        if env_outcome.can_execute() && proto_result.success {
            tests.push(Test::pass(
                "Integration: environment check + protocol send -- full Layer 3 path",
            ));
        } else {
            tests.push(Test::fail(
                "Integration",
                format!("env={} proto={}", env_outcome.label(), proto_result.success),
            ));
        }

        println!("  i  Integration result:");
        println!("     environment: {}", env_outcome.label());
        println!("     protocol:    {}", proto_result.protocol);
        let receipt_preview_len = 8.min(proto_result.receipt_id.len());
        println!(
            "     receipt:     {}",
            &proto_result.receipt_id[..receipt_preview_len]
        );
    }

    // -- RESULTS ----------------------------------------------------
    println!();
    let total = tests.len();
    let passed = tests.iter().filter(|t| t.passed).count();
    let failed = total - passed;

    for t in &tests {
        if t.passed {
            println!("  PASS  {}", t.name);
        } else {
            println!("  FAIL  {}", t.name);
            if let Some(n) = &t.notes {
                println!("           {}", n);
            }
        }
    }

    println!();
    println!("===================================================");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-environment: OK  Any environment. Constraints adapt.");
        println!("  hydra-protocol:    OK  Any protocol. Every event receipted.");
        println!("  COBOL/JCL:         OK  The $10B+ migration pipeline is wired.");
        println!("  Layer 3, Phase 2 complete.");
        println!("  Next: hydra-scheduler -- temporal execution.");
        println!("===================================================");
    }
}
