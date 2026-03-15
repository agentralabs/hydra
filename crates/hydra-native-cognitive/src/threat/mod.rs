//! Predictive Threat Intelligence — P11.
//!
//! Correlates signals from all 17 sisters to detect coordinated threats
//! before they land. Individual sisters handle local defense; this module
//! detects systemic attacks.

pub mod signals;
pub mod correlator;
pub mod scorer;
pub mod responder;

pub use signals::{SisterName, SignalType, ThreatSignal, AttackPattern, SystemBaseline};
pub use correlator::{ThreatAssessment, ThreatLevel, CoordinatedThreat};
pub use responder::ThreatResponse;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use chrono::{Duration, Utc};

/// Maximum signals retained in the rolling buffer.
const MAX_BUFFER_SIZE: usize = 1000;
/// Signals older than this (seconds) are pruned.
const SIGNAL_EXPIRY_SECS: i64 = 3600;

/// Central threat correlation engine.
pub struct ThreatCorrelator {
    signal_buffer: VecDeque<ThreatSignal>,
    known_patterns: Vec<AttackPattern>,
    baseline: SystemBaseline,
    threat_level: AtomicU8,
    assessments: Vec<ThreatAssessment>,
}

impl ThreatCorrelator {
    pub fn new() -> Self {
        Self {
            signal_buffer: VecDeque::new(),
            known_patterns: AttackPattern::known_patterns(),
            baseline: SystemBaseline::default(),
            threat_level: AtomicU8::new(0),
            assessments: Vec::new(),
        }
    }

    /// Ingest a signal from any sister.
    pub fn report_signal(&mut self, signal: ThreatSignal) {
        self.signal_buffer.push_back(signal);
        self.prune_expired();
        // Cap buffer size
        while self.signal_buffer.len() > MAX_BUFFER_SIZE {
            self.signal_buffer.pop_front();
        }
    }

    /// Run correlation sweep — called periodically.
    pub fn correlate(&mut self) -> Vec<ThreatAssessment> {
        self.prune_expired();
        let signals: Vec<ThreatSignal> = self.signal_buffer.iter().cloned().collect();
        let new_assessments = correlator::correlate(&signals, &self.known_patterns);

        // Update threat level from anomaly score
        let score = scorer::anomaly_score(&signals, &self.baseline);
        let numeric = scorer::threat_level_numeric(score);
        self.threat_level.store(numeric, Ordering::Relaxed);

        self.assessments = new_assessments.clone();
        new_assessments
    }

    /// Current threat level (0-10).
    pub fn current_threat_level(&self) -> u8 {
        self.threat_level.load(Ordering::Relaxed)
    }

    /// Current anomaly score.
    pub fn anomaly_score(&self) -> f32 {
        let signals: Vec<ThreatSignal> = self.signal_buffer.iter().cloned().collect();
        scorer::anomaly_score(&signals, &self.baseline)
    }

    /// Respond to a detected threat.
    pub fn respond(&self, assessment: &ThreatAssessment) -> ThreatResponse {
        responder::respond(assessment)
    }

    /// Number of signals in the buffer.
    pub fn signal_count(&self) -> usize {
        self.signal_buffer.len()
    }

    /// Recent assessments from last correlation.
    pub fn recent_assessments(&self) -> &[ThreatAssessment] {
        &self.assessments
    }

    /// Get recent signals for display.
    pub fn recent_signals(&self, limit: usize) -> Vec<&ThreatSignal> {
        self.signal_buffer.iter().rev().take(limit).collect()
    }

    /// Get known attack patterns.
    pub fn known_patterns(&self) -> &[AttackPattern] {
        &self.known_patterns
    }

    /// Summary string for /threat command.
    pub fn summary(&self) -> String {
        let score = self.anomaly_score();
        scorer::threat_summary(score, self.signal_buffer.len(), self.assessments.len())
    }

    /// Detailed signal history for /threat history.
    pub fn signal_history(&self, limit: usize) -> String {
        let signals = self.recent_signals(limit);
        if signals.is_empty() {
            return "No signals recorded.".to_string();
        }
        let mut out = format!("Recent signals ({}):\n", signals.len());
        for sig in signals {
            out.push_str(&format!(
                "  [{}] {} from {} (severity {:.1}): {}\n",
                sig.timestamp.format("%H:%M:%S"),
                sig.signal_type,
                sig.source,
                sig.severity,
                sig.details
            ));
        }
        out
    }

    /// Pattern summary for /threat patterns.
    pub fn patterns_summary(&self) -> String {
        let mut out = format!("Known attack patterns ({}):\n", self.known_patterns.len());
        for pat in &self.known_patterns {
            let types: Vec<&str> = pat.signal_sequence.iter()
                .map(|t| t.as_str())
                .collect();
            out.push_str(&format!(
                "  {} — {} ({}+ sisters, {}s window)\n    Signals: {}\n",
                pat.name, pat.description, pat.min_sisters,
                pat.window_secs, types.join(" → ")
            ));
        }
        out
    }

    /// Remove signals older than the expiry window.
    fn prune_expired(&mut self) {
        let cutoff = Utc::now() - Duration::seconds(SIGNAL_EXPIRY_SECS);
        while let Some(front) = self.signal_buffer.front() {
            if front.timestamp < cutoff {
                self.signal_buffer.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for ThreatCorrelator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
#[path = "threat_tests.rs"]
mod tests;
