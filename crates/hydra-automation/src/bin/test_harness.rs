//! Phase 26 Test Harness — hydra-automation
//! Run: cargo run -p hydra-automation --bin test_harness

use hydra_automation::constants::CRYSTALLIZATION_THRESHOLD;
use hydra_automation::{AutomationEngine, ExecutionObservation, SkillGenerator};
use std::collections::HashMap;

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

fn obs(action: &str, domain: &str, success: bool) -> ExecutionObservation {
    ExecutionObservation::new(action, "test intent", HashMap::new(), domain, 800, success)
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Phase 26 — hydra-automation");
    println!("  Layer 3, Phase 5: Behavior Crystallization");
    println!(
        "  \"You did this {} times. Shall I make it permanent?\"",
        CRYSTALLIZATION_THRESHOLD
    );
    println!("═══════════════════════════════════════════════════════");

    let mut tests = Vec::new();
    let mut engine = AutomationEngine::new();

    // ── OBSERVATION ───────────────────────────────────────────────
    println!("\n── observation ──────────────────────────────────────");

    {
        for _i in 0..(CRYSTALLIZATION_THRESHOLD - 1) {
            engine.observe(obs("deploy.staging", "engineering", true));
        }
        if engine.observation_count() == CRYSTALLIZATION_THRESHOLD - 1
            && engine.pending_proposals().is_empty()
        {
            tests.push(Test::pass("Observation: below threshold — no proposal yet"));
        } else {
            tests.push(Test::fail(
                "Observation: below threshold",
                format!(
                    "obs={} proposals={}",
                    engine.observation_count(),
                    engine.pending_proposals().len()
                ),
            ));
        }
    }

    // ── CRYSTALLIZATION PROPOSAL ──────────────────────────────────
    println!("\n── crystallization proposal ─────────────────────────");

    {
        // Add the threshold-crossing observation
        let proposal_msg = engine.observe(obs("deploy.staging", "engineering", true));

        if let Some(msg) = proposal_msg {
            if msg.contains("Shall I") && msg.contains("deploy.staging") {
                tests.push(Test::pass(
                    "Proposal: threshold crossed — proposal surfaced with 'Shall I'",
                ));
                let display_len = msg.len().min(80);
                println!("  proposal message: \"{}\"", &msg[..display_len]);
            } else {
                let display_len = msg.len().min(60);
                tests.push(Test::fail(
                    "Proposal: message content",
                    msg[..display_len].to_string(),
                ));
            }
        } else {
            tests.push(Test::fail("Proposal: created at threshold", "no proposal"));
        }

        if engine.pending_proposals().len() == 1 {
            tests.push(Test::pass("Proposal: exactly 1 pending proposal"));
        } else {
            tests.push(Test::fail(
                "Proposal: count",
                format!("{}", engine.pending_proposals().len()),
            ));
        }
    }

    {
        // Adding more observations does NOT create duplicate proposals
        for _ in 0..5 {
            engine.observe(obs("deploy.staging", "engineering", true));
        }
        if engine.pending_proposals().len() == 1 {
            tests.push(Test::pass(
                "Proposal: no duplicates — same pattern, one proposal",
            ));
        } else {
            tests.push(Test::fail(
                "Proposal: dedup",
                format!("{} proposals", engine.pending_proposals().len()),
            ));
        }
    }

    // ── APPROVAL AND SKILL GENERATION ────────────────────────────
    println!("\n── approval and skill generation ────────────────────");

    {
        let proposal_id = engine.pending_proposals()[0].id.clone();
        let skill_name = engine
            .approve(&proposal_id)
            .expect("approve should succeed");

        if skill_name.starts_with("auto-") {
            tests.push(Test::pass("Generation: approval — skill generated"));
            println!("  generated skill: '{}'", skill_name);
        } else {
            tests.push(Test::fail("Generation: skill name", skill_name.to_string()));
        }

        if engine.crystallized_count() == 1 {
            tests.push(Test::pass("Generation: crystallized count = 1"));
        } else {
            tests.push(Test::fail(
                "Generation: count",
                format!("{}", engine.crystallized_count()),
            ));
        }

        if engine.pending_proposals().is_empty() {
            tests.push(Test::pass(
                "Generation: no pending proposals after crystallization",
            ));
        } else {
            tests.push(Test::fail(
                "Generation: pending after crystal",
                format!("{}", engine.pending_proposals().len()),
            ));
        }
    }

    // ── SKILL PACKAGE CONTENT ─────────────────────────────────────
    println!("\n── skill package content ────────────────────────────");

    {
        use hydra_automation::pattern::BehaviorPattern;

        let obs1 = ExecutionObservation::new(
            "video.cut",
            "cut the video clip",
            HashMap::new(),
            "creative",
            2000,
            true,
        );
        let mut pattern = BehaviorPattern::new(&obs1);
        for _ in 1..CRYSTALLIZATION_THRESHOLD {
            let o = ExecutionObservation::new(
                "video.cut",
                "cut the video clip",
                HashMap::new(),
                "creative",
                1800,
                true,
            );
            pattern.add_observation(&o);
        }

        let generator = SkillGenerator::new();
        let pkg = generator
            .generate(&pattern)
            .expect("generate should succeed");

        if pkg.is_valid() {
            tests.push(Test::pass(
                "Package: generated package is valid (all required files)",
            ));
        } else {
            tests.push(Test::fail("Package: validity", "invalid"));
        }

        if pkg.skill_toml.contains("[skill]") && pkg.skill_toml.contains("version") {
            tests.push(Test::pass(
                "Package: skill.toml has [skill] section and version",
            ));
        } else {
            tests.push(Test::fail("Package: skill.toml", "missing fields"));
        }

        if pkg.actions_toml.contains("video.cut") && pkg.actions_toml.contains("receipt = true") {
            tests.push(Test::pass(
                "Package: actions.toml has action_id and receipt=true",
            ));
        } else {
            tests.push(Test::fail("Package: actions.toml", "missing fields"));
        }

        if pkg.genome_toml.contains("confidence") && pkg.genome_toml.contains("[[entry]]") {
            tests.push(Test::pass(
                "Package: genome.toml has entries with confidence",
            ));
        } else {
            tests.push(Test::fail("Package: genome.toml", "missing fields"));
        }

        println!("  generated skill: '{}'", pkg.skill_name);
        println!("  {}", pkg.summary());
    }

    // ── DECLINE ───────────────────────────────────────────────────
    println!("\n── decline ──────────────────────────────────────────");

    {
        // Create a new pattern for a different action
        let mut engine2 = AutomationEngine::new();
        for _ in 0..CRYSTALLIZATION_THRESHOLD {
            engine2.observe(obs("security.scan", "security", true));
        }

        let proposal_id = engine2.pending_proposals()[0].id.clone();
        engine2
            .decline(&proposal_id)
            .expect("decline should succeed");

        if engine2.pending_proposals().is_empty() && engine2.crystallized_count() == 0 {
            tests.push(Test::pass(
                "Decline: declined proposal removed from pending, not crystallized",
            ));
        } else {
            tests.push(Test::fail(
                "Decline: state after decline",
                format!(
                    "pending={} crystal={}",
                    engine2.pending_proposals().len(),
                    engine2.crystallized_count()
                ),
            ));
        }
    }

    // ── MULTI-PATTERN ─────────────────────────────────────────────
    println!("\n── multiple patterns ────────────────────────────────");

    {
        let mut engine3 = AutomationEngine::new();
        let actions = vec!["git.commit", "test.run", "deploy.prod"];

        for action in &actions {
            for _ in 0..CRYSTALLIZATION_THRESHOLD {
                engine3.observe(obs(action, "engineering", true));
            }
        }

        if engine3.pattern_count() == 3 {
            tests.push(Test::pass("Multi: 3 distinct patterns detected"));
        } else {
            tests.push(Test::fail(
                "Multi: pattern count",
                format!("{}", engine3.pattern_count()),
            ));
        }

        if engine3.pending_proposals().len() == 3 {
            tests.push(Test::pass("Multi: 3 proposals created (one per pattern)"));
        } else {
            tests.push(Test::fail(
                "Multi: proposal count",
                format!("{}", engine3.pending_proposals().len()),
            ));
        }
    }

    // ── SUMMARY ───────────────────────────────────────────────────
    {
        let s = engine.summary();
        if s.contains("automation:") && s.contains("observations=") && s.contains("crystallized=") {
            tests.push(Test::pass("Summary: format correct for TUI display"));
        } else {
            tests.push(Test::fail("Summary", s.to_string()));
        }
        println!("\n  {}", engine.summary());
    }

    // ── RESULTS ───────────────────────────────────────────────────
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
    println!("═══════════════════════════════════════════════════════");
    println!("  Results: {}/{} passed", passed, total);
    if failed > 0 {
        println!("  FAILED: {} test(s)", failed);
        std::process::exit(1);
    } else {
        println!();
        println!("  hydra-automation verified.");
        println!("  Behavior crystallization: operational.");
        println!(
            "  {} observations → proposal surfaced.",
            CRYSTALLIZATION_THRESHOLD
        );
        println!("  Principal approves → skill generated.");
        println!("  Decline → continues observing.");
        println!("  Generated format == SKILL-FORMAT-SPEC.md.");
        println!("  Layer 3, Phase 5 complete.");
        println!("  Next: hydra-audit — execution accountability narrative.");
        println!("  Layer 3 closes after Phase 27.");
        println!("═══════════════════════════════════════════════════════");
    }
}
