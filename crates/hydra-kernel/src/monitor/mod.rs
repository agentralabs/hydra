//! O16 Omniscient Monitor — unified event system watching all data sources.
//! Pollers (pull), watchers (local), correlator, auto-actions, classifier.
//! Events flow to TUI stream with priority indicators.

pub mod auto_action;
pub mod correlator;
pub mod events;
pub mod poller;
pub mod watcher;

pub use events::{MonitorEvent, EventPriority, EventCategory, classify_event, redact_sensitive};
pub use poller::{Poller, PollerSource};
pub use watcher::{LocalWatcher, WatcherResult};
pub use correlator::{Correlator, Correlation};
pub use auto_action::{AutoAction, AutoActionEngine};

use crate::loop_::middleware::CycleMiddleware;
use crate::loop_::types::{CycleResult, PerceivedInput};

// ── MonitorHub — central coordinator ──

/// Central coordinator for all monitors — pollers, watchers, auto-actions, correlator.
pub struct MonitorHub {
    pollers: Vec<Poller>,
    watchers: Vec<LocalWatcher>,
    event_buffer: Vec<MonitorEvent>,
    auto_actions: AutoActionEngine,
    correlator: Correlator,
    last_watcher_states: std::collections::HashMap<String, String>,
}

impl MonitorHub {
    pub fn new() -> Self {
        Self {
            pollers: Vec::new(),
            watchers: Vec::new(),
            event_buffer: Vec::new(),
            auto_actions: AutoActionEngine::new(),
            correlator: Correlator::new(),
            last_watcher_states: std::collections::HashMap::new(),
        }
    }

    pub fn add_poller(&mut self, poller: Poller) {
        eprintln!("hydra-monitor: added poller '{}'", poller.name);
        self.pollers.push(poller);
    }

    pub fn add_watcher(&mut self, watcher: LocalWatcher) {
        eprintln!("hydra-monitor: added watcher '{}'", watcher.source_name());
        self.watchers.push(watcher);
    }

    pub fn add_auto_action(&mut self, action: AutoAction) {
        self.auto_actions.add_action(action);
    }

    /// Tick all monitors — poll, check watchers, classify, correlate, run auto-actions.
    pub fn tick(&mut self) -> Vec<MonitorEvent> {
        let mut new_events = Vec::new();
        let genome = hydra_genome::GenomeStore::open();

        // Poll all ready pollers
        for poller in &mut self.pollers {
            if poller.should_poll() {
                if let Some(mut event) = poller.poll() {
                    event.priority = classify_event(&event.category, &event.detail, &genome);
                    self.correlator.add_event(&event);
                    // Check auto-actions
                    if let Some(action) = self.auto_actions.check_event(&event) {
                        event.detail = format!("{} [auto-action: {action}]", event.detail);
                    }
                    new_events.push(event);
                }
            }
        }

        // Check all watchers
        for watcher in &self.watchers {
            let result = watcher.check();
            let source = watcher.source_name();
            let prev_state = self.last_watcher_states.get(&source);
            let state_changed = prev_state.map(|p| p != &result.detail).unwrap_or(result.changed);
            self.last_watcher_states.insert(source, result.detail.clone());
            if state_changed {
                if let Some(mut event) = watcher.to_event(&WatcherResult { changed: true, ..result }) {
                    event.priority = classify_event(&event.category, &event.detail, &genome);
                    self.correlator.add_event(&event);
                    new_events.push(event);
                }
            }
        }

        self.event_buffer.extend(new_events.clone());
        new_events
    }

    /// Get all pending events (drain buffer).
    pub fn drain_events(&mut self) -> Vec<MonitorEvent> {
        std::mem::take(&mut self.event_buffer)
    }

    /// Count of alert-priority events in buffer.
    pub fn alert_count(&self) -> usize {
        self.event_buffer.iter().filter(|e| e.priority == EventPriority::Alert).count()
    }

    /// Total number of active monitors.
    pub fn monitor_count(&self) -> usize {
        self.pollers.iter().filter(|p| p.enabled).count() + self.watchers.len()
    }

    /// Recent events for display.
    pub fn recent_events(&self, limit: usize) -> Vec<&MonitorEvent> {
        self.event_buffer.iter().rev().take(limit).collect()
    }
}

impl Default for MonitorHub {
    fn default() -> Self { Self::new() }
}

// ── MonitorMiddleware ──

/// Monitor middleware — injects pending events into the cognitive loop.
pub struct MonitorMiddleware {
    hub: MonitorHub,
}

impl MonitorMiddleware {
    pub fn new() -> Self {
        Self { hub: MonitorHub::new() }
    }
}

impl CycleMiddleware for MonitorMiddleware {
    fn name(&self) -> &'static str { "monitor" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Tick monitors and collect new events
        let events = self.hub.tick();
        if events.is_empty() { return; }
        // Inject alerts and important events as enrichments
        let alerts: Vec<String> = events.iter()
            .filter(|e| e.priority == EventPriority::Alert || e.priority == EventPriority::Important)
            .map(|e| e.format_brief())
            .collect();
        if !alerts.is_empty() {
            perceived.enrichments.insert("monitor_events".into(), alerts.join("; "));
        }
    }

    fn post_deliver(&mut self, cycle: &CycleResult) {
        // Record which events user engaged with (genome learning)
        if cycle.response.contains("monitor") || cycle.response.contains("alert") {
            let mut genome = hydra_genome::GenomeStore::open();
            for event in self.hub.recent_events(5) {
                let desc = format!("monitor:{} {}", event.source, event.category.label());
                let approach = hydra_genome::ApproachSignature::new(
                    "monitor_engagement", vec![event.title.clone()], vec!["monitor".into()]);
                if let Err(e) = genome.add_from_operation(&desc, approach, 0.6) {
                    eprintln!("hydra-monitor: genome write failed: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_creates() {
        let hub = MonitorHub::new();
        assert_eq!(hub.monitor_count(), 0);
        assert_eq!(hub.alert_count(), 0);
    }

    #[test]
    fn add_poller_increments_count() {
        let mut hub = MonitorHub::new();
        hub.add_poller(Poller::new("test", PollerSource::PortCheck { port: 9999, expect_open: false }, 60));
        assert_eq!(hub.monitor_count(), 1);
    }

    #[test]
    fn add_watcher_increments_count() {
        let mut hub = MonitorHub::new();
        hub.add_watcher(LocalWatcher::Port { port: 9999, expect_open: false });
        assert_eq!(hub.monitor_count(), 1);
    }

    #[test]
    fn empty_tick_no_events() {
        let mut hub = MonitorHub::new();
        let events = hub.tick();
        assert!(events.is_empty());
    }

    #[test]
    fn middleware_name() {
        let mw = MonitorMiddleware::new();
        assert_eq!(mw.name(), "monitor");
    }
}
