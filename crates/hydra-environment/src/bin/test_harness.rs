//! Test harness for hydra-environment.

use hydra_environment::{
    check_requirements, CheckOutcome, EnvironmentEngine, RequiredBinary, SkillRequirements,
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
    println!("=== hydra-environment test harness ===\n");
    let mut tests = Vec::new();

    // Test 1: Probe detects local machine
    {
        let mut engine = EnvironmentEngine::new();
        let profile = engine.probe().expect("probe failed");
        if profile.capabilities.cpu_cores > 0 && profile.capabilities.ram_mb > 0 {
            tests.push(Test::pass("probe detects CPU cores and RAM"));
        } else {
            tests.push(Test::fail(
                "probe",
                format!(
                    "cores={} ram={}",
                    profile.capabilities.cpu_cores, profile.capabilities.ram_mb
                ),
            ));
        }
        println!("  info: {}", engine.summary());
    }

    // Test 2: Full capability when RAM sufficient
    {
        let mut engine = EnvironmentEngine::new();
        let reqs = SkillRequirements::new("hello-skill");
        engine.register_requirements(reqs);
        engine.probe().expect("probe failed");
        let outcome = engine.check_skill("hello-skill").expect("check failed");
        if outcome.can_execute() {
            tests.push(Test::pass("minimal skill requirements -> can execute"));
        } else {
            tests.push(Test::fail("minimal skill", "blocked"));
        }
    }

    // Test 3: Missing required binary -> blocked
    {
        use hydra_environment::profile::{
            EnvironmentCapabilities, EnvironmentClass, EnvironmentProfile, OsType,
        };
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
            tests.push(Test::pass("missing required binary -> blocked"));
        } else {
            tests.push(Test::fail("binary blocking", "not blocked"));
        }
    }

    // Test 4: Degraded when low RAM threshold
    {
        use hydra_environment::profile::{
            EnvironmentCapabilities, EnvironmentClass, EnvironmentProfile, OsType,
        };
        let profile_low_ram = EnvironmentProfile::new(
            EnvironmentClass::LocalMachine,
            EnvironmentCapabilities {
                ram_mb: 300,
                disk_mb: 100_000,
                cpu_cores: 4,
                has_gpu: false,
                has_network: true,
                has_filesystem: true,
                os_type: OsType::detect(),
            },
        );
        let mut reqs = SkillRequirements::new("ram-hungry");
        reqs.min_ram_mb = 1024;
        reqs.low_resource_threshold_mb = 200;
        let outcome = check_requirements(&reqs, &profile_low_ram);
        match &outcome {
            CheckOutcome::DegradedCapability { .. } => {
                tests.push(Test::pass("low RAM -> degraded mode"));
            }
            other => {
                tests.push(Test::fail(
                    "low RAM degraded",
                    format!("expected degraded, got: {}", other.label()),
                ));
            }
        }
    }

    // Test 5: Unknown skill error
    {
        let mut engine = EnvironmentEngine::new();
        let result = engine.check_skill("nonexistent-skill");
        if result.is_err() {
            tests.push(Test::pass("unknown skill returns error"));
        } else {
            tests.push(Test::fail("unknown skill", "did not error"));
        }
    }

    // Summary
    let passed = tests.iter().filter(|t| t.passed).count();
    let total = tests.len();
    println!();
    for (i, t) in tests.iter().enumerate() {
        let status = if t.passed { "PASS" } else { "FAIL" };
        let note = t
            .notes
            .as_ref()
            .map(|n| format!(" ({n})"))
            .unwrap_or_default();
        println!("  Test {}: {} — {}{}", i + 1, status, t.name, note);
    }
    println!("\n=== {passed}/{total} environment tests passed ===");

    if passed < total {
        std::process::exit(1);
    }
}
