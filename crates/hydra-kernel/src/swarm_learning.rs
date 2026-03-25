//! Swarm learning bridge — connects dream loop to parallel web intelligence.
//! Single-agent learning always runs. Swarm learning activates when idle.
//!
//! Architecture: LearningLoop (single-agent, always) + hydra-swarm-browser
//! (multi-agent, when idle >300s). Results feed through learning_validator
//! into genome.

use hydra_genome::{ApproachSignature, GenomeStore};

use crate::learning_loop::{LearningLoop, LearningResult};
use crate::learning_validator::{self, ValidationResult};

/// How often swarm learning activates (in dream steps).
const SWARM_INTERVAL: u64 = 500;

/// Minimum idle seconds before swarm triggers (EC-20.10: don't interfere with conversation).
const MIN_IDLE_SECS: u64 = 300;

/// Maximum genome entries added per swarm run.
const MAX_SWARM_ENTRIES: usize = 20;

pub struct SwarmLearning {
    single: LearningLoop,
    last_swarm_step: u64,
}

impl SwarmLearning {
    pub fn new() -> Self {
        Self { single: LearningLoop::new(), last_swarm_step: 0 }
    }

    /// Run one tick. Always runs single-agent. Conditionally runs swarm.
    pub fn tick(&mut self, genome: &mut GenomeStore, step: u64, idle_secs: u64) -> LearningResult {
        // Always run single-agent learning
        let mut result = self.single.tick(genome);

        // Check swarm conditions
        if !should_swarm(step, self.last_swarm_step, idle_secs) {
            return result;
        }
        self.last_swarm_step = step;

        let pool_size = adaptive_pool_size();
        let goal_desc = pick_swarm_goal(genome);
        if goal_desc.is_empty() { return result; }

        eprintln!("hydra-swarm-learning: launching {} workers for '{}'", pool_size, goal_desc);

        // Execute swarm (blocking — dream loop has no urgency)
        match hydra_swarm_browser::execute_swarm_blocking(&goal_desc) {
            Ok(display) => {
                // Parse the merged text into genome entries
                let added = ingest_swarm_results(&display, &goal_desc, genome);
                result.entries_added += added;
                eprintln!("hydra-swarm-learning: +{added} genome entries from swarm");
            }
            Err(e) => eprintln!("hydra-swarm-learning: {e}"),
        }

        result
    }
}

impl Default for SwarmLearning {
    fn default() -> Self { Self::new() }
}

fn should_swarm(step: u64, last: u64, idle_secs: u64) -> bool {
    step.saturating_sub(last) >= SWARM_INTERVAL && idle_secs >= MIN_IDLE_SECS
}

/// RAM-adaptive pool sizing (EC-20.5).
fn adaptive_pool_size() -> usize {
    #[cfg(target_os = "macos")]
    {
        let mut size: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        let name = std::ffi::CString::new("hw.memsize").unwrap();
        unsafe {
            libc::sysctlbyname(name.as_ptr(), &mut size as *mut u64 as *mut _, &mut len, std::ptr::null_mut(), 0);
        }
        let gb = size / (1024 * 1024 * 1024);
        if gb <= 8 { 2 } else if gb <= 16 { 3 } else { 5 }
    }
    #[cfg(not(target_os = "macos"))]
    { 3 }
}

/// Pick a learning goal from genome gaps — domain with lowest avg confidence.
fn pick_swarm_goal(genome: &GenomeStore) -> String {
    let stats = genome.domain_stats();
    if stats.is_empty() { return String::new(); }
    // Find domain with lowest average confidence (biggest knowledge gap)
    let weakest = stats.iter()
        .min_by(|a, b| a.avg_confidence.partial_cmp(&b.avg_confidence).unwrap_or(std::cmp::Ordering::Equal));
    match weakest {
        Some(d) if d.avg_confidence < 0.7 => format!("learn about {}", d.domain),
        _ => String::new(), // No significant gaps
    }
}

/// Split swarm merged text into sentences and validate each as a genome candidate.
fn ingest_swarm_results(merged_text: &str, domain: &str, genome: &mut GenomeStore) -> usize {
    let mut added = 0;
    for sentence in merged_text.split(|c: char| c == '.' || c == '\n') {
        let trimmed = sentence.trim();
        if trimmed.len() < 30 || added >= MAX_SWARM_ENTRIES { continue; }

        match learning_validator::validate(trimmed, domain, genome) {
            ValidationResult::Novel => {
                let approach = ApproachSignature::new("swarm-harvested", vec![trimmed.to_string()], vec![]);
                match genome.add(hydra_genome::GenomeEntry::from_operation(trimmed, approach, 0.55)) {
                    Ok(_) => added += 1,
                    Err(e) => eprintln!("hydra-swarm-learning: genome add: {e}"),
                }
            }
            ValidationResult::Complementary { existing_id } => {
                let _ = genome.record_use(&existing_id, true);
            }
            ValidationResult::Conflict { existing_id } => {
                learning_validator::save_conflict(trimmed, &existing_id, domain);
            }
            ValidationResult::Duplicate { .. } => {} // Skip
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_swarm_respects_interval() {
        assert!(!should_swarm(100, 0, 400)); // too few steps
        assert!(should_swarm(600, 0, 400));  // enough steps + idle
        assert!(!should_swarm(600, 0, 100)); // not idle enough
    }

    #[test]
    fn adaptive_pool_returns_valid_range() {
        let size = adaptive_pool_size();
        assert!(size >= 2 && size <= 5);
    }

    #[test]
    fn pick_goal_empty_genome() {
        let genome = GenomeStore::new();
        let goal = pick_swarm_goal(&genome);
        assert!(goal.is_empty());
    }
}
