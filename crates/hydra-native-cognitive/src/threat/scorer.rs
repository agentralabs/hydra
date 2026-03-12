//! Anomaly scoring — how far current state deviates from baseline.

use super::signals::{SystemBaseline, ThreatSignal};
use super::correlator::ThreatLevel;
use chrono::{Duration, Utc};

/// Calculate anomaly score (0.0 = normal, 1.0 = maximally anomalous).
pub fn anomaly_score(signals: &[ThreatSignal], baseline: &SystemBaseline) -> f32 {
    if signals.is_empty() {
        return 0.0;
    }

    let now = Utc::now();
    let hour_ago = now - Duration::hours(1);

    // Signals in the last hour
    let recent: Vec<&ThreatSignal> = signals.iter()
        .filter(|s| s.timestamp >= hour_ago)
        .collect();

    if recent.is_empty() {
        return 0.0;
    }

    // Factor 1: Volume deviation (how many more signals than expected?)
    let volume_ratio = recent.len() as f32 / baseline.avg_signals_per_hour.max(0.1);
    let volume_score = (volume_ratio - 1.0).max(0.0).min(1.0);

    // Factor 2: Severity deviation (how much more severe than normal?)
    let avg_severity: f32 = recent.iter()
        .map(|s| s.weighted_severity())
        .sum::<f32>() / recent.len() as f32;
    let severity_score = ((avg_severity - baseline.avg_severity) / 0.5).max(0.0).min(1.0);

    // Factor 3: Novel signal types (types not in normal baseline?)
    let novel_count = recent.iter()
        .filter(|s| !baseline.normal_signal_types.contains(&s.signal_type))
        .count();
    let novelty_score = (novel_count as f32 / recent.len() as f32).min(1.0);

    // Weighted combination
    let score = volume_score * 0.3 + severity_score * 0.4 + novelty_score * 0.3;
    score.clamp(0.0, 1.0)
}

/// Convert a numeric threat level (0-10) from anomaly score.
pub fn threat_level_numeric(score: f32) -> u8 {
    (score * 10.0).round() as u8
}

/// Convert anomaly score to threat level.
pub fn score_to_level(score: f32) -> ThreatLevel {
    ThreatLevel::from_score(score)
}

/// Summary string for current threat posture.
pub fn threat_summary(
    score: f32,
    signal_count: usize,
    recent_assessments: usize,
) -> String {
    let level = score_to_level(score);
    let numeric = threat_level_numeric(score);

    match level {
        ThreatLevel::None => format!(
            "Threat Level: {}/10 (None)\n{} signals ingested, {} assessments. All clear.",
            numeric, signal_count, recent_assessments
        ),
        ThreatLevel::Low => format!(
            "Threat Level: {}/10 (Low)\n{} signals, {} assessments. Minor anomalies detected.",
            numeric, signal_count, recent_assessments
        ),
        ThreatLevel::Medium => format!(
            "Threat Level: {}/10 (Medium)\n{} signals, {} assessments. Elevated activity.",
            numeric, signal_count, recent_assessments
        ),
        ThreatLevel::High => format!(
            "Threat Level: {}/10 (High)\n{} signals, {} assessments. Active threat detected.",
            numeric, signal_count, recent_assessments
        ),
        ThreatLevel::Critical => format!(
            "Threat Level: {}/10 (Critical)\n{} signals, {} assessments. LOCKDOWN recommended.",
            numeric, signal_count, recent_assessments
        ),
    }
}
