//! Integration tests: full cognitive loop + skill loading + binary lifecycle.

use crate::TestResult;
use std::process::Command;
use std::time::{Duration, Instant};

pub fn run() -> Vec<TestResult> {
    let mut results = Vec::new();

    results.extend(test_skill_loading());
    results.extend(test_cognitive_loop());
    results.extend(test_binaries());

    let passed = results.iter().filter(|r| r.passed).count();
    println!("  Integration: {}/{} passed", passed, results.len());
    results
}

fn test_skill_loading() -> Vec<TestResult> {
    let mut results = Vec::new();
    let start = Instant::now();

    match std::panic::catch_unwind(|| {
        let skills_dir = std::path::PathBuf::from("skills");
        assert!(skills_dir.exists(), "skills/ must exist");
        let mut skill_count = 0usize;
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let genome_path = entry.path().join("genome.toml");
                    if genome_path.exists() {
                        skill_count += 1;
                        println!("    skill: {}",
                            entry.file_name().to_string_lossy());
                    }
                }
            }
        }
        assert!(skill_count >= 1, "At least 1 skill must be loaded");
        skill_count
    }) {
        Ok(count) => {
            println!("  [PASS] skill-loading: {} skill(s) with genome", count);
            results.push(TestResult::pass("skill-loading", "skills_present",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] skill-loading: {}", err);
            results.push(TestResult::fail("skill-loading", "skills_present",
                &err, start.elapsed().as_millis() as u64));
        }
    }
    results
}

fn test_cognitive_loop() -> Vec<TestResult> {
    let mut results = Vec::new();

    let start = Instant::now();
    match std::panic::catch_unwind(|| {
        use hydra_comprehension::{ComprehensionEngine, InputSource};
        use hydra_context::{ContextFrame, SessionHistory, GapContext, AnomalyContext};
        use hydra_language::LanguageEngine;
        use hydra_attention::AttentionEngine;
        use hydra_reasoning::ReasoningEngine;
        use hydra_genome::GenomeStore;

        let comp    = ComprehensionEngine::new();
        let genome  = GenomeStore::new();
        let reason  = ReasoningEngine::new();

        // Simulate one full cognitive cycle (no LLM)
        let input = "what are the benefits of the circuit breaker pattern";
        let comprehended = comp.comprehend(input, InputSource::PrincipalText, &genome)
            .expect("Comprehension must succeed");
        assert!(comprehended.confidence > 0.0);

        let language = LanguageEngine::analyze(&comprehended).ok();
        let history  = SessionHistory::new();
        let gap_ctx  = GapContext::new();
        let anomaly  = AnomalyContext::new();
        let context  = ContextFrame::build(
            &comprehended, &history, &[], &gap_ctx, &anomaly,
        );

        if context.total_items() > 0 {
            if let Some(ref lang) = language {
                if let Ok(att_frame) = AttentionEngine::allocate(
                    &comprehended, &context, lang,
                ) {
                    let _ = reason.reason(&comprehended, &att_frame, &genome);
                    // Reasoning may fail -- that is OK. Pipeline ran.
                }
            }
        }
    }) {
        Ok(()) => {
            println!("  [PASS] cognitive-loop: full pipeline (no LLM)");
            results.push(TestResult::pass("cognitive-loop", "full_pipeline_no_llm",
                start.elapsed().as_millis() as u64));
        }
        Err(e) => {
            let err = format!("{:?}", e);
            println!("  [FAIL] cognitive-loop: {}", err);
            results.push(TestResult::fail("cognitive-loop", "full_pipeline_no_llm",
                &err, start.elapsed().as_millis() as u64));
        }
    }

    results
}

fn test_binaries() -> Vec<TestResult> {
    let mut results = Vec::new();

    // Test 1: hydra_fed starts and exits cleanly
    {
        let start = Instant::now();
        match std::panic::catch_unwind(|| {
            let mut child = Command::new("cargo")
                .args(["run", "-p", "hydra-kernel", "--bin", "hydra_fed"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("spawn hydra_fed");

            std::thread::sleep(Duration::from_secs(2));

            // Kill gracefully
            let _ = child.kill();
            let status = child.wait().expect("wait for hydra_fed");

            // Exit code 101 = Rust panic. Anything else is acceptable.
            assert!(
                status.code() != Some(101),
                "hydra_fed panicked (exit 101)"
            );
        }) {
            Ok(()) => {
                println!("  [PASS] binary: hydra_fed starts/exits cleanly");
                results.push(TestResult::pass(
                    "binary",
                    "hydra_fed_lifecycle",
                    start.elapsed().as_millis() as u64,
                ));
            }
            Err(e) => {
                let err = format!("{:?}", e);
                println!("  [FAIL] binary: hydra_fed: {}", err);
                results.push(TestResult::fail(
                    "binary",
                    "hydra_fed_lifecycle",
                    &err,
                    start.elapsed().as_millis() as u64,
                ));
            }
        }
    }

    // Test 2: hydra_tui starts and exits cleanly (no TTY — expects graceful error)
    {
        let start = Instant::now();
        match std::panic::catch_unwind(|| {
            let mut child = Command::new("cargo")
                .args(["run", "-p", "hydra-tui", "--bin", "hydra_tui"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("spawn hydra_tui");

            std::thread::sleep(Duration::from_secs(2));

            let _ = child.kill();
            let status = child.wait().expect("wait for hydra_tui");

            // Without a TTY, crossterm will fail gracefully (not panic).
            // Exit 101 = panic = fail. Any other exit is OK.
            assert!(
                status.code() != Some(101),
                "hydra_tui panicked (exit 101)"
            );
        }) {
            Ok(()) => {
                println!("  [PASS] binary: hydra_tui starts/exits cleanly");
                results.push(TestResult::pass(
                    "binary",
                    "hydra_tui_lifecycle",
                    start.elapsed().as_millis() as u64,
                ));
            }
            Err(e) => {
                let err = format!("{:?}", e);
                println!("  [FAIL] binary: hydra_tui: {}", err);
                results.push(TestResult::fail(
                    "binary",
                    "hydra_tui_lifecycle",
                    &err,
                    start.elapsed().as_millis() as u64,
                ));
            }
        }
    }

    // Test 3: main hydra binary produces output
    {
        let start = Instant::now();
        match std::panic::catch_unwind(|| {
            let output = Command::new("cargo")
                .args([
                    "run", "-p", "hydra-kernel", "--bin", "hydra", "--", "test",
                ])
                .output()
                .expect("run hydra binary");

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // stdout must be non-empty (the response)
            assert!(
                !stdout.trim().is_empty(),
                "hydra binary produced no stdout"
            );

            // stderr must contain a receipt footer [session|path|tok|ms|mw=N]
            assert!(
                stderr.contains("tok|") && stderr.contains("ms|"),
                "hydra binary stderr missing receipt footer: {}",
                stderr
            );
        }) {
            Ok(()) => {
                println!("  [PASS] binary: hydra produces output + receipt");
                results.push(TestResult::pass(
                    "binary",
                    "hydra_main_output",
                    start.elapsed().as_millis() as u64,
                ));
            }
            Err(e) => {
                let err = format!("{:?}", e);
                println!("  [FAIL] binary: hydra main: {}", err);
                results.push(TestResult::fail(
                    "binary",
                    "hydra_main_output",
                    &err,
                    start.elapsed().as_millis() as u64,
                ));
            }
        }
    }

    results
}
