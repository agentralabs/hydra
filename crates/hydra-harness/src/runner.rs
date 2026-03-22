//! runner.rs — Executes one full sweep of all crates.

use crate::layers;

pub struct HarnessRun {
    pub hour:    u32,
    pub results: Vec<crate::TestResult>,
    pub started: chrono::DateTime<chrono::Utc>,
    pub ended:   chrono::DateTime<chrono::Utc>,
}

impl HarnessRun {
    pub fn passed(&self) -> usize { self.results.iter().filter(|r| r.passed).count() }
    pub fn failed(&self) -> usize { self.results.iter().filter(|r| !r.passed).count() }
    pub fn fixed(&self)  -> usize {
        self.results.iter()
            .filter(|r| r.fix_succeeded == Some(true))
            .count()
    }
    pub fn total(&self) -> usize { self.results.len() }
}

/// Run all tests for all crates across all layers.
pub fn run_all(hour: u32) -> HarnessRun {
    let started = chrono::Utc::now();
    let mut results = Vec::new();

    println!("\n======================================================");
    println!("  HOUR {:02} -- FULL CAPABILITY SWEEP", hour);
    println!("======================================================\n");

    println!("-- LAYER 1 ------------------------------------------");
    results.extend(layers::layer1::run());

    println!("-- LAYER 2 ------------------------------------------");
    results.extend(layers::layer2::run());

    println!("-- LAYER 3 ------------------------------------------");
    results.extend(layers::layer3::run());

    println!("-- LAYER 4 ------------------------------------------");
    results.extend(layers::layer4::run());

    println!("-- LAYER 5 ------------------------------------------");
    results.extend(layers::layer5::run());

    println!("-- LAYER 6 ------------------------------------------");
    results.extend(layers::layer6::run());

    println!("-- LAYER 7 ------------------------------------------");
    results.extend(layers::layer7::run());

    println!("-- INTEGRATION --------------------------------------");
    results.extend(layers::integration::run());

    println!("-- NEW SUBSYSTEMS -----------------------------------");
    results.extend(layers::new_subsystems::run());

    let ended = chrono::Utc::now();

    HarnessRun { hour, results, started, ended }
}
