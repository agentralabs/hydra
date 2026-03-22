//! harness_v2.rs — The behavioral intelligence test loop.
//! Run: cargo run -p hydra-harness --bin harness_v2
//! Or:  cargo run -p hydra-harness --bin harness_v2 -- --hours 3

use hydra_harness::v2::{
    bank::{question_bank, variation_bank},
    runner::run_hydra,
    evaluator::{grade_question, grade_variation},
    analyzer::{analyze, HourlyData, Severity},
    reporter::{write_hourly, write_grand, V2Run},
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total_hours: u32 = args.iter()
        .position(|a| a == "--hours")
        .and_then(|i| args.get(i + 1))
        .and_then(|h| h.parse().ok())
        .unwrap_or(10);

    // Load from environment or .env file
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        // Try loading from .env file
        let env_paths = [".env", "../.env", "../../.env"];
        for path in &env_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                for line in content.lines() {
                    if let Some(val) = line.strip_prefix("ANTHROPIC_API_KEY=") {
                        return val.trim().to_string();
                    }
                }
            }
        }
        panic!("ANTHROPIC_API_KEY not found in environment or .env file");
    });

    // Check if V1 may be running (boot lock warning)
    let lock_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".hydra")
        .join("hydra.lock");
    if lock_path.exists() {
        eprintln!("Warning: hydra.lock exists — V1 may be running.");
        eprintln!("V2 will retry on lock collisions automatically.");
    }

    let questions  = question_bank();
    let variations = variation_bank();

    println!("===========================================================");
    println!("  HYDRA HARNESS V2 -- BEHAVIORAL INTELLIGENCE");
    println!("  {} hours | 12 questions | 12 variations", total_hours);
    println!("  Grader: claude-haiku (fast + cheap)");
    println!("===========================================================\n");

    let mut all_runs   = Vec::new();
    let mut all_hourly = Vec::new();

    for hour in 1..=total_hours {
        println!("\n-- Hour {:02} -----------------------------------------", hour);
        let mut hour_scores = Vec::new();

        // VARIANT 1: Question bank
        println!("  Variant 1: Questions ({} inputs)...", questions.len());
        for q in &questions {
            print!("    {} ... ", q.id);
            let response = run_hydra(q.text, &api_key);
            let score    = grade_question(&response, q, &api_key, hour).await;
            println!("{:.1}/10 -- {}", score.score, score.finding);
            hour_scores.push(score);
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        // VARIANT 2: Variation bank
        println!("  Variant 2: Variations ({} inputs)...", variations.len());
        for v in &variations {
            print!("    {} ({:?}) ... ", v.variant_id, v.formality);
            let response = run_hydra(v.text, &api_key);
            let score    = grade_variation(&response, v, &api_key).await;
            println!("{:.1}/10 -- {}", score.score, score.finding);
            hour_scores.push(score.clone());
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        let avg = hour_scores.iter().map(|s| s.score).sum::<f64>()
                / hour_scores.len() as f64;
        println!("\n  Hour {:02} avg: {:.1}/10", hour, avg);

        let run = V2Run { hour, scores: hour_scores.clone() };
        write_hourly(&run);
        all_runs.push(run);
        all_hourly.push(HourlyData { hour, scores: hour_scores });

        if hour < total_hours {
            println!("  Next run in 1 hour...");
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    // Analyze and write grand report
    println!("\n  Analyzing findings...");
    let findings = analyze(&all_hourly);

    println!("\n  Findings:");
    for f in &findings {
        let icon = match f.severity {
            Severity::Critical => "[CRITICAL]",
            Severity::Issue    => "[ISSUE]",
            Severity::Advisory => "[ADVISORY]",
            Severity::Healthy  => "[HEALTHY]",
        };
        println!("  {} {}", icon, f.name);
    }

    write_grand(&all_runs, &findings);

    println!("\n===========================================================");
    println!("  V2 COMPLETE");
    println!("  Review harness-reports/GRAND-REPORT-V2.md");
    println!("===========================================================");
}
