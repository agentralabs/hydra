//! O16 Omniscient Monitor — unified event system watching all data sources.
//! Pollers (pull), watchers (local), correlator, auto-actions, classifier.
//! Events flow to TUI stream with priority indicators.

pub mod auto_action;
pub mod connectors;
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
                    // Check auto-actions — execute with 10s timeout + killpg
                    if let Some(action) = self.auto_actions.check_event(&event) {
                        let mut cmd = std::process::Command::new("sh");
                        cmd.arg("-c").arg(&action)
                            .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
                        #[cfg(unix)]
                        unsafe { use std::os::unix::process::CommandExt; cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) }); }
                        match cmd.spawn() {
                            Ok(mut child) => {
                                let pgid = child.id() as i32;
                                let (tx, rx) = std::sync::mpsc::channel();
                                std::thread::spawn(move || { let _ = tx.send(child.wait_with_output()); });
                                match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                                    Ok(Ok(out)) => {
                                        let stdout = String::from_utf8_lossy(&out.stdout);
                                        let status = if out.status.success() { "OK" } else { "FAILED" };
                                        event.detail = format!("{} [auto-action {status}: {action}]", event.detail);
                                        eprintln!("hydra-monitor: auto-action: {}", &stdout[..stdout.len().min(200)]);
                                    }
                                    _ => {
                                        #[cfg(unix)]
                                        unsafe { libc::killpg(pgid, libc::SIGKILL); }
                                        event.detail = format!("{} [auto-action TIMEOUT: {action}]", event.detail);
                                        eprintln!("hydra-monitor: auto-action timeout (10s), killed pgid {pgid}");
                                    }
                                }
                            }
                            Err(e) => {
                                event.detail = format!("{} [auto-action ERROR: {e}]", event.detail);
                                eprintln!("hydra-monitor: auto-action exec failed: {e}");
                            }
                        }
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

        // EC-16.2: Cap event buffer to prevent OOM on high-volume sources
        const MAX_EVENT_BUFFER: usize = 500;
        if self.event_buffer.len() + new_events.len() > MAX_EVENT_BUFFER {
            let drain = (self.event_buffer.len() + new_events.len()) - MAX_EVENT_BUFFER;
            self.event_buffer.drain(..drain.min(self.event_buffer.len()));
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
    active_connectors: Vec<connectors::ActiveConnector>,
}

impl MonitorMiddleware {
    pub fn new() -> Self {
        let active_connectors = connectors::load_connectors();
        if !active_connectors.is_empty() {
            eprintln!("hydra-monitor: loaded {} connectors", active_connectors.len());
        }
        Self { hub: MonitorHub::new(), active_connectors }
    }
}

impl CycleMiddleware for MonitorMiddleware {
    fn name(&self) -> &'static str { "monitor" }

    fn post_perceive(&mut self, perceived: &mut PerceivedInput) {
        // Tick monitors and collect new events
        let mut events = self.hub.tick();
        // Poll active connectors (database, API, cloud)
        for conn in &mut self.active_connectors {
            if conn.should_poll() { events.extend(conn.poll()); }
        }
        if events.is_empty() { return; }
        // Inject alerts and important events as enrichments
        let alerts: Vec<String> = events.iter()
            .filter(|e| e.priority == EventPriority::Alert || e.priority == EventPriority::Important)
            .map(|e| e.format_brief())
            .collect();
        if !alerts.is_empty() {
            perceived.enrichments.insert("monitor_events".into(), alerts.join("; "));
        }
        // Persist alert/important events for proactive engine consumption
        let trigger_events: Vec<crate::proactive::MonitorTriggerEvent> = events.iter()
            .filter(|e| e.priority == EventPriority::Alert || e.priority == EventPriority::Important)
            .map(|e| crate::proactive::MonitorTriggerEvent {
                title: e.title.clone(),
                category: e.category.label().into(),
                detail: e.detail.clone(),
                urgency: if e.priority == EventPriority::Alert { 0.9 } else { 0.6 },
                suggested_action: format!("Investigate: {}", e.title),
                timestamp: chrono::Utc::now(),
            }).collect();
        if !trigger_events.is_empty() {
            let events_dir = dirs::home_dir().unwrap_or_default().join(".hydra/monitor");
            let _ = std::fs::create_dir_all(&events_dir);
            let events_path = events_dir.join("events.json");
            match serde_json::to_string(&trigger_events) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&events_path, json) {
                        eprintln!("hydra-monitor: failed to persist events: {e}");
                    }
                }
                Err(e) => eprintln!("hydra-monitor: serialize events failed: {e}"),
            }
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
