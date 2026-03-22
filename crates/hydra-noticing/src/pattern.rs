//! PatternWatcher — tracks recurring behaviors and notices when they break.
//! "You usually deploy on Mondays. It's been 14 days since the last one."

use crate::{
    constants::PATTERN_BREAK_DAYS,
    signal::{NoticingKind, NoticingSignal},
};
use std::collections::HashMap;

/// A recurring pattern being watched.
#[derive(Debug, Clone)]
pub struct WatchedPattern {
    pub name:                   String,
    pub occurrences:            Vec<chrono::DateTime<chrono::Utc>>,
    pub expected_interval_days: f64,
}

impl WatchedPattern {
    pub fn new(name: impl Into<String>, expected_interval_days: f64) -> Self {
        Self {
            name: name.into(),
            occurrences: Vec::new(),
            expected_interval_days,
        }
    }

    pub fn record_occurrence(&mut self) {
        self.occurrences.push(chrono::Utc::now());
    }

    pub fn last_occurrence(&self) -> Option<&chrono::DateTime<chrono::Utc>> {
        self.occurrences.last()
    }

    pub fn days_since_last(&self) -> Option<u64> {
        self.last_occurrence()
            .map(|t| (chrono::Utc::now() - *t).num_days() as u64)
    }

    pub fn is_broken(&self) -> bool {
        match self.days_since_last() {
            None => false, // never occurred — not yet a pattern
            Some(d) => {
                d >= PATTERN_BREAK_DAYS
                    && d > (self.expected_interval_days * 1.5) as u64
            }
        }
    }
}

/// Watches multiple recurring patterns.
#[derive(Debug, Default)]
pub struct PatternWatcher {
    patterns: HashMap<String, WatchedPattern>,
}

impl PatternWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn watch(&mut self, name: impl Into<String>, expected_interval_days: f64) {
        let n = name.into();
        self.patterns
            .entry(n.clone())
            .or_insert_with(|| WatchedPattern::new(n, expected_interval_days));
    }

    pub fn record(&mut self, pattern_name: &str) {
        if let Some(p) = self.patterns.get_mut(pattern_name) {
            p.record_occurrence();
        }
    }

    /// Check all patterns for breaks and generate signals.
    pub fn check_for_breaks(&self) -> Vec<NoticingSignal> {
        self.patterns
            .values()
            .filter(|p| p.is_broken())
            .filter_map(|p| {
                let days = p.days_since_last()?;
                let last = *p.last_occurrence()?;
                let significance =
                    ((days as f64 / p.expected_interval_days) - 1.0).clamp(0.0, 1.0);

                Some(NoticingSignal::new(
                    NoticingKind::PatternBreak {
                        pattern:       p.name.clone(),
                        last_occurred: last,
                        days_absent:   days,
                    },
                    significance * 0.8,
                    format!(
                        "Noticed: '{}' pattern has not occurred in {} days. \
                         Expected every {:.0} days. Last: {} days ago.",
                        p.name, days, p.expected_interval_days, days
                    ),
                    Some(format!("Consider: is '{}' still relevant?", p.name)),
                ))
            })
            .collect()
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_not_broken_when_recent() {
        let mut p = WatchedPattern::new("daily-deploy", 1.0);
        p.record_occurrence();
        assert!(!p.is_broken());
    }

    #[test]
    fn pattern_not_broken_when_never_occurred() {
        let p = WatchedPattern::new("new-pattern", 7.0);
        assert!(!p.is_broken());
    }

    #[test]
    fn watcher_tracks_multiple_patterns() {
        let mut w = PatternWatcher::new();
        w.watch("deploy", 7.0);
        w.watch("security-scan", 1.0);
        assert_eq!(w.pattern_count(), 2);
    }

    #[test]
    fn no_breaks_for_fresh_patterns() {
        let mut w = PatternWatcher::new();
        w.watch("deploy", 7.0);
        w.record("deploy");
        let breaks = w.check_for_breaks();
        assert!(breaks.is_empty());
    }
}
