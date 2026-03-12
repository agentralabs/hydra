//! Pattern matching across multi-sister signals.

use std::collections::{HashMap, HashSet};
use chrono::{Duration, Utc};

use super::signals::{AttackPattern, SisterName, SignalType, ThreatSignal};

/// Result of a correlation sweep.
#[derive(Debug, Clone)]
pub struct ThreatAssessment {
    pub threat_level: ThreatLevel,
    pub description: String,
    pub contributing_signals: Vec<ThreatSignal>,
    pub matched_pattern: Option<String>,
    pub confidence: f32,
}

/// Graduated threat levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl ThreatLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Critical => "Critical",
        }
    }

    pub fn from_score(score: f32) -> Self {
        if score >= 0.8 { Self::Critical }
        else if score >= 0.6 { Self::High }
        else if score >= 0.4 { Self::Medium }
        else if score >= 0.2 { Self::Low }
        else { Self::None }
    }
}

impl std::fmt::Display for ThreatLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A coordinated threat detected across multiple sisters.
#[derive(Debug, Clone)]
pub struct CoordinatedThreat {
    pub sisters_involved: Vec<SisterName>,
    pub signal_count: usize,
    pub severity: f32,
    pub window_secs: u64,
}

/// Run correlation across a signal buffer.
pub fn correlate(
    signals: &[ThreatSignal],
    patterns: &[AttackPattern],
) -> Vec<ThreatAssessment> {
    let mut assessments = Vec::new();

    // Check each known pattern against recent signals
    for pattern in patterns {
        if let Some(assessment) = match_pattern(signals, pattern) {
            assessments.push(assessment);
        }
    }

    // Check for general coordinated attack (3+ sisters in 5 sec window)
    if let Some(coordinated) = detect_coordinated_attack(signals, 5) {
        let already_matched = assessments.iter().any(|a| {
            a.threat_level >= ThreatLevel::High
        });
        if !already_matched {
            assessments.push(ThreatAssessment {
                threat_level: ThreatLevel::High,
                description: format!(
                    "Coordinated attack: {} sisters reporting {} signals in {}s",
                    coordinated.sisters_involved.len(),
                    coordinated.signal_count,
                    coordinated.window_secs
                ),
                contributing_signals: signals.iter()
                    .filter(|s| coordinated.sisters_involved.contains(&s.source))
                    .cloned()
                    .collect(),
                matched_pattern: Some("Coordinated Attack".into()),
                confidence: coordinated.severity,
            });
        }
    }

    assessments
}

/// Match a specific attack pattern against recent signals.
fn match_pattern(
    signals: &[ThreatSignal],
    pattern: &AttackPattern,
) -> Option<ThreatAssessment> {
    let now = Utc::now();
    let window = Duration::seconds(pattern.window_secs as i64);

    // Get signals within the pattern's time window
    let recent: Vec<&ThreatSignal> = signals.iter()
        .filter(|s| now.signed_duration_since(s.timestamp) <= window)
        .collect();

    // Check if required signal types are present
    let mut matched_types = Vec::new();
    let mut sisters_seen = HashSet::new();

    for required_type in &pattern.signal_sequence {
        if let Some(sig) = recent.iter().find(|s| {
            &s.signal_type == required_type && !matched_types.contains(&s.timestamp)
        }) {
            matched_types.push(sig.timestamp);
            sisters_seen.insert(sig.source.clone());
        }
    }

    // All signal types matched and enough sisters involved?
    if matched_types.len() >= pattern.signal_sequence.len()
        && sisters_seen.len() >= pattern.min_sisters
    {
        let avg_severity: f32 = recent.iter()
            .map(|s| s.weighted_severity())
            .sum::<f32>() / recent.len().max(1) as f32;

        let threat_level = if avg_severity >= 0.7 {
            ThreatLevel::Critical
        } else if avg_severity >= 0.5 {
            ThreatLevel::High
        } else {
            ThreatLevel::Medium
        };

        Some(ThreatAssessment {
            threat_level,
            description: format!("Pattern '{}': {}", pattern.name, pattern.description),
            contributing_signals: recent.into_iter().cloned().collect(),
            matched_pattern: Some(pattern.name.clone()),
            confidence: avg_severity,
        })
    } else {
        None
    }
}

/// Detect coordinated attacks: 3+ sisters reporting within `window_secs`.
pub fn detect_coordinated_attack(
    signals: &[ThreatSignal],
    window_secs: u64,
) -> Option<CoordinatedThreat> {
    let now = Utc::now();
    let window = Duration::seconds(window_secs as i64);

    let recent: Vec<&ThreatSignal> = signals.iter()
        .filter(|s| now.signed_duration_since(s.timestamp) <= window)
        .collect();

    let mut sisters: HashMap<&SisterName, usize> = HashMap::new();
    for sig in &recent {
        *sisters.entry(&sig.source).or_insert(0) += 1;
    }

    if sisters.len() >= 3 {
        let avg_severity: f32 = recent.iter()
            .map(|s| s.weighted_severity())
            .sum::<f32>() / recent.len().max(1) as f32;

        Some(CoordinatedThreat {
            sisters_involved: sisters.keys().map(|&s| s.clone()).collect(),
            signal_count: recent.len(),
            severity: avg_severity,
            window_secs,
        })
    } else {
        None
    }
}
