//! harness.rs — The main autonomous test loop.
//! Run: cargo run -p hydra-harness --bin harness
//! Or:  cargo run -p hydra-harness --bin harness -- --hours 3

use hydra_harness::{runner, reporter, fixer};
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let total_hours: u32 = args.iter()
        .position(|a| a == "--hours")
        .and_then(|i| args.get(i + 1))
        .and_then(|h| h.parse().ok())
        .unwrap_or(20);

    println!("==========================================================");
    println!("  HYDRA AUTONOMOUS TEST HARNESS");
    println!("  {} hours | All crates | Every capability", total_hours);
    println!();
    println!("  Reports: harness-reports/HOUR-XX.md");
    println!("  Grand:   harness-reports/GRAND-REPORT.md");
    println!("==========================================================\n");

    let mut all_runs = Vec::new();

    for hour in 1..=total_hours {
        let mut run = runner::run_all(hour);

        // Attempt fixes for failures
        let failed: Vec<_> = run.results.iter()
            .enumerate()
            .filter(|(_, r)| !r.passed)
            .map(|(i, _)| i)
            .collect();

        if !failed.is_empty() {
            println!("\n  Attempting fixes for {} failures...", failed.len());
            for idx in failed {
                fixer::attempt_fix(&mut run.results[idx]);

                if run.results[idx].fix_succeeded == Some(true) {
                    println!("    Fixed: {}::{}",
                        run.results[idx].crate_name,
                        run.results[idx].capability);
                }
            }
        }

        // Print hour summary
        println!("\n  === Hour {:02} Summary ===", hour);
        println!("  Passed:     {}", run.passed());
        println!("  Failed:     {}", run.failed());
        println!("  Fixed:      {}", run.fixed());
        if run.total() > 0 {
            println!("  Pass rate:  {:.1}%",
                run.passed() as f64 / run.total() as f64 * 100.0);
        }

        reporter::write_hourly_report(&run);
        all_runs.push(run);

        // Wait until next hour (unless last hour)
        if hour < total_hours {
            println!("\n  Next run in 1 hour...\n");
            std::thread::sleep(Duration::from_secs(3600));
        }
    }

    // Write grand report
    println!("\n\n  Writing grand report...");
    reporter::write_grand_report(&all_runs);

    println!("\n==========================================================");
    println!("  HARNESS COMPLETE");
    println!("  Review harness-reports/GRAND-REPORT.md");
    println!("==========================================================");
}
