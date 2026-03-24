//! O16 Auto-Action Engine — pre-approved automatic responses to monitor events.
//! EC-16.5: Circuit breaker prevents cascade failures.

use std::collections::HashMap;
use std::time::Instant;

use super::events::MonitorEvent;

/// A pre-approved automatic response to a specific event trigger.
#[derive(Debug, Clone)]
pub struct AutoAction {
    pub trigger: String,
    pub condition: String,
    pub action: String,
    pub description: String,
    pub enabled: bool,
}

/// Engine that checks events against auto-actions with circuit breaker protection.
pub struct AutoActionEngine {
    actions: Vec<AutoAction>,
    /// EC-16.5: Circuit breaker — action_trigger → (fire_count, first_fire_time)
    circuit_breaker: HashMap<String, (u32, Instant)>,
}

const CIRCUIT_BREAKER_MAX_FIRES: u32 = 3;
const CIRCUIT_BREAKER_WINDOW_SECS: u64 = 600; // 10 minutes

impl AutoActionEngine {
    pub fn new() -> Self {
        Self { actions: Vec::new(), circuit_breaker: HashMap::new() }
    }

    /// Add an auto-action rule.
    pub fn add_action(&mut self, action: AutoAction) {
        self.actions.push(action);
    }

    /// Check an event against all auto-actions. Returns the action to execute if matched.
    pub fn check_event(&mut self, event: &MonitorEvent) -> Option<String> {
        for action in &self.actions {
            if !action.enabled { continue; }
            // Match trigger against event source
            if !event.source.contains(&action.trigger) && !event.title.to_lowercase().contains(&action.trigger) {
                continue;
            }
            // Check condition (simple substring match for now — LLM would evaluate complex conditions)
            if !action.condition.is_empty() && !event.detail.to_lowercase().contains(&action.condition.to_lowercase()) {
                continue;
            }
            // EC-16.5: Check circuit breaker
            if self.circuit_open(&action.trigger) {
                eprintln!("hydra-monitor: auto-action '{}' circuit open — pausing", action.trigger);
                return None;
            }
            // Clone to release borrow before mutable call
            let trigger = action.trigger.clone();
            let action_str = action.action.clone();
            self.record_fire(&trigger);
            eprintln!("hydra-monitor: auto-action fired: {trigger} → {action_str}");
            return Some(action_str);
        }
        None
    }

    /// EC-16.5: Check if circuit breaker is open (too many fires in window).
    fn circuit_open(&self, trigger: &str) -> bool {
        if let Some((count, first_time)) = self.circuit_breaker.get(trigger) {
            if first_time.elapsed().as_secs() < CIRCUIT_BREAKER_WINDOW_SECS {
                return *count >= CIRCUIT_BREAKER_MAX_FIRES;
            }
        }
        false
    }

    /// Record a fire for circuit breaker tracking.
    fn record_fire(&mut self, trigger: &str) {
        let entry = self.circuit_breaker.entry(trigger.to_string()).or_insert((0, Instant::now()));
        if entry.1.elapsed().as_secs() >= CIRCUIT_BREAKER_WINDOW_SECS {
            // Window expired — reset
            *entry = (1, Instant::now());
        } else {
            entry.0 += 1;
        }
    }

    pub fn action_count(&self) -> usize { self.actions.len() }
}

impl Default for AutoActionEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::events::{EventCategory, MonitorEvent};

    fn make_event(source: &str, title: &str, detail: &str) -> MonitorEvent {
        MonitorEvent::new(source, EventCategory::Infrastructure, title, detail)
    }

    #[test]
    fn action_matches_trigger() {
        let mut engine = AutoActionEngine::new();
        engine.add_action(AutoAction {
            trigger: "cpu_high".into(), condition: "".into(),
            action: "scale_up".into(), description: "Auto-scale".into(), enabled: true,
        });
        let event = make_event("cpu_high", "CPU spike", "cpu at 95%");
        let result = engine.check_event(&event);
        assert_eq!(result, Some("scale_up".into()));
    }

    #[test]
    fn no_match_returns_none() {
        let mut engine = AutoActionEngine::new();
        engine.add_action(AutoAction {
            trigger: "cpu_high".into(), condition: "".into(),
            action: "scale_up".into(), description: "Auto-scale".into(), enabled: true,
        });
        let event = make_event("email", "New email", "from alice");
        assert!(engine.check_event(&event).is_none());
    }

    #[test]
    fn circuit_breaker_triggers() {
        let mut engine = AutoActionEngine::new();
        engine.add_action(AutoAction {
            trigger: "cpu_high".into(), condition: "".into(),
            action: "scale_up".into(), description: "Auto-scale".into(), enabled: true,
        });
        let event = make_event("cpu_high", "CPU spike", "cpu at 95%");
        // Fire 3 times
        assert!(engine.check_event(&event).is_some());
        assert!(engine.check_event(&event).is_some());
        assert!(engine.check_event(&event).is_some());
        // 4th should be blocked by circuit breaker
        assert!(engine.check_event(&event).is_none());
    }
}
