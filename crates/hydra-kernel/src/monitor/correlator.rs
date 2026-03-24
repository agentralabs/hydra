//! O16 Temporal Correlator — connects events across sources within time windows.
//! EC-16.4: Only reports correlations above 0.7 confidence.
//! EC-16.10: All timestamps normalized to UTC.

use std::collections::VecDeque;
use super::events::MonitorEvent;

const MAX_EVENTS: usize = 100;
const DEFAULT_WINDOW_SECS: u64 = 600; // 10 minutes

/// A detected correlation between two events.
#[derive(Debug, Clone)]
pub struct Correlation {
    pub event_a_source: String,
    pub event_b_source: String,
    pub confidence: f64,
    pub description: String,
}

/// Temporal event correlator — finds related events in recent history.
pub struct Correlator {
    recent_events: VecDeque<MonitorEvent>,
    window_secs: u64,
}

impl Correlator {
    pub fn new() -> Self {
        Self { recent_events: VecDeque::with_capacity(MAX_EVENTS), window_secs: DEFAULT_WINDOW_SECS }
    }

    /// Add an event to the correlation buffer.
    pub fn add_event(&mut self, event: &MonitorEvent) {
        if self.recent_events.len() >= MAX_EVENTS {
            self.recent_events.pop_front();
        }
        self.recent_events.push_back(event.clone());
    }

    /// Find correlations between the given event and recent history.
    /// EC-16.4: Only returns correlations with confidence > 0.7.
    pub fn find_correlations(&self, event: &MonitorEvent) -> Vec<Correlation> {
        let mut correlations = Vec::new();
        let event_time = event.timestamp;

        for prev in &self.recent_events {
            if prev.id == event.id { continue; }
            // EC-16.10: timestamps already in UTC (chrono::Utc)
            let time_diff = (event_time - prev.timestamp).num_seconds().unsigned_abs();
            if time_diff > self.window_secs { continue; }

            // Compute correlation confidence based on:
            // 1. Time proximity (closer = higher)
            // 2. Category relationship (infra+cicd = high, same source = low — that's just the same thing)
            let time_factor = 1.0 - (time_diff as f64 / self.window_secs as f64);
            let category_factor = category_relatedness(&prev.category, &event.category);
            // Same source = not a cross-source correlation
            if prev.source == event.source { continue; }

            let confidence = (time_factor * 0.6 + category_factor * 0.4).clamp(0.0, 1.0);

            // EC-16.4: only report if above threshold
            if confidence > 0.7 {
                correlations.push(Correlation {
                    event_a_source: prev.source.clone(),
                    event_b_source: event.source.clone(),
                    confidence,
                    description: format!(
                        "{} ({}) MAY be related to {} ({}) — {:.0}% confidence, {}s apart",
                        prev.title, prev.source, event.title, event.source,
                        confidence * 100.0, time_diff
                    ),
                });
            }
        }
        correlations
    }

    /// Number of events in the buffer.
    pub fn event_count(&self) -> usize { self.recent_events.len() }
}

impl Default for Correlator {
    fn default() -> Self { Self::new() }
}

/// How related two event categories are (0.0-1.0).
fn category_relatedness(a: &super::events::EventCategory, b: &super::events::EventCategory) -> f64 {
    use super::events::EventCategory::*;
    match (a, b) {
        (Infrastructure, CiCd) | (CiCd, Infrastructure) => 0.9,
        (Infrastructure, Security) | (Security, Infrastructure) => 0.8,
        (CiCd, Project) | (Project, CiCd) => 0.7,
        (a, b) if a == b => 0.5, // Same category but different source
        _ => 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::events::{EventCategory, MonitorEvent};

    fn make_event(source: &str, cat: EventCategory, title: &str) -> MonitorEvent {
        MonitorEvent::new(source, cat, title, "detail")
    }

    #[test]
    fn no_correlation_for_distant_events() {
        let mut c = Correlator::new();
        let mut old = make_event("github", EventCategory::CiCd, "build failed");
        old.timestamp = chrono::Utc::now() - chrono::Duration::hours(1);
        c.add_event(&old);
        let new = make_event("server", EventCategory::Infrastructure, "cpu high");
        let corrs = c.find_correlations(&new);
        assert!(corrs.is_empty());
    }

    #[test]
    fn correlates_close_events() {
        let mut c = Correlator::new();
        let deploy = make_event("github", EventCategory::CiCd, "deployed v2.3");
        c.add_event(&deploy);
        let cpu = make_event("server", EventCategory::Infrastructure, "cpu spike");
        let corrs = c.find_correlations(&cpu);
        // CiCd + Infrastructure = high category relatedness, close in time
        assert!(!corrs.is_empty(), "Expected correlation between deploy and cpu spike");
        assert!(corrs[0].confidence > 0.7);
    }

    #[test]
    fn same_source_not_correlated() {
        let mut c = Correlator::new();
        let e1 = make_event("github", EventCategory::CiCd, "build 1");
        c.add_event(&e1);
        let e2 = make_event("github", EventCategory::CiCd, "build 2");
        let corrs = c.find_correlations(&e2);
        assert!(corrs.is_empty());
    }
}
