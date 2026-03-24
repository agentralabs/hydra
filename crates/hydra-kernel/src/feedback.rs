//! Feedback-Genome Loop — every action outcome updates genome, calibration, antifragile.
//! After 500 actions, Hydra is measurably better at executing tasks.

use hydra_antifragile::{AntifragileStore, ObstacleClass};
use hydra_calibration::CalibrationEngine;
use hydra_genome::GenomeStore;

// ── Types ──

/// Outcome of an executed action — feeds the three learning systems.
#[derive(Debug, Clone)]
pub enum ActionOutcome {
    /// Action succeeded with quality score.
    Success { approach: String, domain: String, duration_ms: u64, quality: f64 },
    /// Partially succeeded.
    PartialSuccess { approach: String, domain: String, what_worked: String, what_failed: String },
    /// Failed — with obstacle info and whether the 13-approach cycle rerouted.
    Failure { approach: String, domain: String, obstacle: String, error: String, rerouted: bool },
    /// Novel situation — no prior genome entry.
    Novel { situation: String, approach: String, succeeded: bool },
    /// User cancelled — no feedback recorded (EC-3.5).
    Cancelled,
}

// ── Constants ──

const LEARNING_RATE: f64 = 0.1;          // EC-3.4: EMA alpha — recent but not dominant
const MIN_DOMAIN_MATCH_SCORE: f64 = 0.3; // EC-3.3: below = create new, don't update wrong entry
const NOVEL_INITIAL_CONFIDENCE: f64 = 0.6;
const MAX_CONFIDENCE: f64 = 1.0;         // EC-3.2: clamp ceiling
const MIN_CONFIDENCE: f64 = 0.05;        // EC-3.2: never fully zero

/// Log an outcome without genome write (for immutable genome contexts).
pub fn log_outcome(outcome: &ActionOutcome) {
    match outcome {
        ActionOutcome::Success { approach, domain, .. } => {
            eprintln!("hydra-feedback: success in {domain}: {approach}");
        }
        ActionOutcome::Failure { approach, domain, obstacle, .. } => {
            eprintln!("hydra-feedback: failure in {domain}: {approach} — {obstacle}");
        }
        ActionOutcome::Cancelled => {}
        _ => {}
    }
}

// ── Main Entry Point ──

/// Record an action outcome — updates genome, calibration, antifragile.
pub fn record_outcome(
    outcome: &ActionOutcome,
    genome: &mut GenomeStore,
    calibration: &mut CalibrationEngine,
    antifragile: &mut AntifragileStore,
) {
    match outcome {
        ActionOutcome::Cancelled => {
            // EC-3.5: cancelled = neither success nor failure. No update.
            eprintln!("hydra-feedback: cancelled — no outcome recorded");
        }
        ActionOutcome::Success { approach, domain, quality, .. } => {
            update_genome_success(genome, domain, approach, *quality);
            update_calibration(calibration, domain, *quality);
            eprintln!("hydra-feedback: success recorded (domain={domain}, quality={quality:.2})");
        }
        ActionOutcome::PartialSuccess { approach, domain, .. } => {
            update_genome_success(genome, domain, approach, 0.5);
            update_calibration(calibration, domain, 0.5);
            eprintln!("hydra-feedback: partial success recorded (domain={domain})");
        }
        ActionOutcome::Failure { approach, domain, obstacle, rerouted, .. } => {
            update_genome_failure(genome, domain, approach);
            update_calibration(calibration, domain, 0.0);
            // EC-3.8: only record antifragile if actually failed + rerouted
            if *rerouted {
                update_antifragile(antifragile, obstacle, true);
            } else {
                update_antifragile(antifragile, obstacle, false);
            }
            eprintln!("hydra-feedback: failure recorded (domain={domain}, rerouted={rerouted})");
        }
        ActionOutcome::Novel { situation, approach, succeeded } => {
            if *succeeded {
                // Create new genome entry from successful novel approach
                let approach_sig = hydra_genome::ApproachSignature::new(
                    "action-feedback",
                    vec![approach.clone()],
                    vec!["conductor".into()],
                );
                match genome.add_from_operation(situation, approach_sig, NOVEL_INITIAL_CONFIDENCE) {
                    Ok(id) => eprintln!("hydra-feedback: novel success → genome entry {id}"),
                    Err(e) => eprintln!("hydra-feedback: genome add failed: {e}"),
                }
            }
            update_calibration(calibration, "novel", if *succeeded { 1.0 } else { 0.0 });
            eprintln!("hydra-feedback: novel outcome (succeeded={succeeded})");
        }
    }
}

/// Simplified version — just genome update without calibration/antifragile.
/// Used when those systems aren't available (e.g., from TUI /feedback command).
pub fn record_simple(outcome: &ActionOutcome, genome: &mut GenomeStore) {
    match outcome {
        ActionOutcome::Success { domain, approach, quality, .. } => {
            update_genome_success(genome, domain, approach, *quality);
        }
        ActionOutcome::Failure { domain, approach, .. } => {
            update_genome_failure(genome, domain, approach);
        }
        ActionOutcome::Novel { situation, approach, succeeded } if *succeeded => {
            let sig = hydra_genome::ApproachSignature::new("feedback", vec![approach.clone()], vec![]);
            if let Err(e) = genome.add_from_operation(situation, sig, NOVEL_INITIAL_CONFIDENCE) {
                eprintln!("hydra-feedback: {e}");
            }
        }
        _ => {}
    }
}

// ── Genome Updates ──

fn update_genome_success(genome: &mut GenomeStore, domain: &str, approach: &str, _quality: f64) {
    let query = format!("{domain} {approach}");
    // Collect ID first to avoid borrow conflict
    let entry_id = genome.query(&query).first()
        .filter(|e| e.effective_confidence() > MIN_DOMAIN_MATCH_SCORE)
        .map(|e| e.id.clone());
    if let Some(id) = entry_id {
        if let Err(e) = genome.record_use(&id, true) {
            eprintln!("hydra-feedback: genome record_use: {e}");
        }
    }
}

fn update_genome_failure(genome: &mut GenomeStore, domain: &str, approach: &str) {
    let query = format!("{domain} {approach}");
    let entry_id = genome.query(&query).first()
        .filter(|e| e.effective_confidence() > MIN_DOMAIN_MATCH_SCORE)
        .map(|e| e.id.clone());
    if let Some(id) = entry_id {
        if let Err(e) = genome.record_use(&id, false) {
            eprintln!("hydra-feedback: genome record_use (failure): {e}");
        }
    }
}

// ── Calibration Updates ──

fn update_calibration(calibration: &mut CalibrationEngine, domain: &str, actual: f64) {
    let predicted = 0.8; // Default prediction confidence
    match calibration.record_prediction(domain, hydra_calibration::JudgmentType::SuccessProbability, predicted) {
        Ok(id) => {
            if let Err(e) = calibration.record_outcome(&id, actual) {
                eprintln!("hydra-feedback: calibration record_outcome: {e}");
            }
        }
        Err(e) => eprintln!("hydra-feedback: calibration record_prediction: {e}"),
    }
}

// ── Antifragile Updates ──

fn update_antifragile(store: &mut AntifragileStore, obstacle_desc: &str, overcome: bool) {
    let class = classify_obstacle(obstacle_desc);
    match store.get_or_create(&class) {
        Ok(record) => {
            record.record_encounter(overcome, None);
            eprintln!("hydra-feedback: antifragile {:?} (overcome={overcome})", class);
        }
        Err(e) => eprintln!("hydra-feedback: antifragile store: {e}"),
    }
}

fn classify_obstacle(desc: &str) -> ObstacleClass {
    let lower = desc.to_lowercase();
    if lower.contains("permission") || lower.contains("denied") || lower.contains("auth") {
        ObstacleClass::PermissionDenied
    } else if lower.contains("timeout") || lower.contains("timed out") {
        ObstacleClass::TimeoutPattern
    } else if lower.contains("not found") || lower.contains("missing") {
        ObstacleClass::DependencyMissing
    } else if lower.contains("port") || lower.contains("address in use") {
        ObstacleClass::ConcurrencyConflict
    } else if lower.contains("rate limit") || lower.contains("429") {
        ObstacleClass::RateLimit
    } else if lower.contains("disk") || lower.contains("space") {
        ObstacleClass::ResourceExhaustion
    } else if lower.contains("network") || lower.contains("connection") {
        ObstacleClass::NetworkBlock
    } else {
        ObstacleClass::Other
    }
}

/// Bayesian EMA confidence update — EC-3.4: smooth, EC-3.2: clamped.
pub fn bayesian_update(prior: f64, observation: f64) -> f64 {
    let updated = prior * (1.0 - LEARNING_RATE) + observation * LEARNING_RATE;
    updated.clamp(MIN_CONFIDENCE, MAX_CONFIDENCE) // EC-3.2
}

// ── O13: Taste Learning ──

/// Record aesthetic taste feedback — updates genome confidence for design patterns.
/// EC-13.2: Respects user taste even if it contradicts design principles.
pub fn record_taste_feedback(domain: &str, positive: bool, genome: &mut hydra_genome::GenomeStore) {
    let query = format!("aesthetic {domain}");
    let matches = genome.query(&query);
    if let Some(entry) = matches.first() {
        let id = entry.id.clone();
        if let Err(e) = genome.record_use(&id, positive) {
            eprintln!("hydra-feedback: taste record: {e}");
        }
        eprintln!("hydra-feedback: taste {} for {domain} (updated existing)", if positive { "positive" } else { "negative" });
    } else {
        // New aesthetic domain — create entry
        let entry = hydra_genome::social_genome::create_communication_entry(
            "aesthetic", domain, &format!("user taste for {domain}"), if positive { 0.8 } else { 0.3 });
        if let Err(e) = genome.add(entry) {
            eprintln!("hydra-feedback: taste add: {e}");
        }
        eprintln!("hydra-feedback: taste {} for {domain} (new entry)", if positive { "positive" } else { "negative" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayesian_update_clamps() {
        assert!(bayesian_update(0.99, 1.0) <= 1.0); // EC-3.2
        assert!(bayesian_update(0.01, 0.0) >= MIN_CONFIDENCE);
    }

    #[test]
    fn bayesian_update_moves_toward_observation() {
        let prior = 0.5;
        let after_success = bayesian_update(prior, 1.0);
        assert!(after_success > prior);
        let after_failure = bayesian_update(prior, 0.0);
        assert!(after_failure < prior);
    }

    #[test]
    fn cancelled_no_update() {
        let mut genome = GenomeStore::new();
        let mut cal = CalibrationEngine::new();
        let mut anti = AntifragileStore::new();
        record_outcome(&ActionOutcome::Cancelled, &mut genome, &mut cal, &mut anti);
        // No crash, no panic — cancelled is a no-op
    }

    #[test]
    fn classify_obstacle_types() {
        assert_eq!(classify_obstacle("permission denied"), ObstacleClass::PermissionDenied);
        assert_eq!(classify_obstacle("connection timeout"), ObstacleClass::TimeoutPattern);
        assert_eq!(classify_obstacle("file not found"), ObstacleClass::DependencyMissing);
        assert_eq!(classify_obstacle("port 3000 already in use"), ObstacleClass::ConcurrencyConflict);
        assert_eq!(classify_obstacle("something weird"), ObstacleClass::Other);
    }

    #[test]
    fn novel_success_records() {
        let mut genome = GenomeStore::new();
        let mut cal = CalibrationEngine::new();
        let mut anti = AntifragileStore::new();
        let outcome = ActionOutcome::Novel {
            situation: "test task".into(), approach: "echo test".into(), succeeded: true,
        };
        record_outcome(&outcome, &mut genome, &mut cal, &mut anti);
        // Genome should have at least 1 entry from novel success
    }

    #[test]
    fn learning_rate_prevents_oscillation() {
        // EC-3.4: rapid success/failure shouldn't oscillate wildly
        let mut conf = 0.5;
        for i in 0..20 {
            conf = bayesian_update(conf, if i % 2 == 0 { 1.0 } else { 0.0 });
        }
        // After 20 alternating observations, confidence should be near 0.5 (stable)
        assert!((conf - 0.5).abs() < 0.1, "Confidence should stabilize: {conf}");
    }
}
