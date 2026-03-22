//! Temporal horizons — how far into the future the soul considers.

use serde::{Deserialize, Serialize};

/// The temporal horizon classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalHorizon {
    /// Immediate concerns (days to weeks).
    Immediate,
    /// Developmental concerns (months to a year).
    Developmental,
    /// Foundational concerns (years).
    Foundational,
    /// Generational concerns (decades+).
    Generational,
}

impl TemporalHorizon {
    /// Returns the care multiplier for this horizon.
    ///
    /// Longer horizons produce higher multipliers because they reflect
    /// deeper caring about outcomes.
    pub fn care_multiplier(&self) -> f64 {
        match self {
            Self::Immediate => 1.0,
            Self::Developmental => 1.5,
            Self::Foundational => 2.0,
            Self::Generational => 3.0,
        }
    }
}

/// Signals that determine the temporal horizon classification.
#[derive(Debug, Clone, Default)]
pub struct TemporalSignals {
    /// Number of immediate-horizon signals observed.
    pub immediate_count: u64,
    /// Number of developmental-horizon signals observed.
    pub developmental_count: u64,
    /// Number of foundational-horizon signals observed.
    pub foundational_count: u64,
    /// Number of generational-horizon signals observed.
    pub generational_count: u64,
}

impl TemporalSignals {
    /// Classify the dominant temporal horizon based on signal counts.
    ///
    /// Returns the horizon with the highest signal count.
    /// Ties break toward longer horizons (more care).
    pub fn classify(&self) -> TemporalHorizon {
        let total = self.immediate_count
            + self.developmental_count
            + self.foundational_count
            + self.generational_count;

        // No signals at all — default to immediate.
        if total == 0 {
            return TemporalHorizon::Immediate;
        }

        // Ties break toward longer horizons (higher priority value).
        let counts = [
            (self.immediate_count, 0u64, TemporalHorizon::Immediate),
            (self.developmental_count, 1, TemporalHorizon::Developmental),
            (self.foundational_count, 2, TemporalHorizon::Foundational),
            (self.generational_count, 3, TemporalHorizon::Generational),
        ];

        counts
            .into_iter()
            .max_by_key(|(count, priority, _)| (*count, *priority))
            .map(|(_, _, horizon)| horizon)
            .unwrap_or(TemporalHorizon::Immediate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_immediate() {
        let signals = TemporalSignals::default();
        assert_eq!(signals.classify(), TemporalHorizon::Immediate);
    }

    #[test]
    fn highest_count_wins() {
        let signals = TemporalSignals {
            immediate_count: 1,
            developmental_count: 5,
            foundational_count: 2,
            generational_count: 0,
        };
        assert_eq!(signals.classify(), TemporalHorizon::Developmental);
    }

    #[test]
    fn ties_break_toward_longer() {
        let signals = TemporalSignals {
            immediate_count: 3,
            developmental_count: 3,
            foundational_count: 3,
            generational_count: 3,
        };
        // Generational should win on tie (checked first in ordered list)
        assert_eq!(signals.classify(), TemporalHorizon::Generational);
    }

    #[test]
    fn care_multipliers_increase() {
        assert!(
            TemporalHorizon::Developmental.care_multiplier()
                > TemporalHorizon::Immediate.care_multiplier()
        );
        assert!(
            TemporalHorizon::Foundational.care_multiplier()
                > TemporalHorizon::Developmental.care_multiplier()
        );
        assert!(
            TemporalHorizon::Generational.care_multiplier()
                > TemporalHorizon::Foundational.care_multiplier()
        );
    }
}
