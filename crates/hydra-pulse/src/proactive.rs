//! ProactiveEngine — push updates without the user asking.

use serde::{Deserialize, Serialize};

/// What triggers a proactive update
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProactiveTrigger {
    /// A watched file changed
    FileChanged { path: String },
    /// A scheduled check fired
    ScheduledCheck { name: String },
    /// A sister reported something noteworthy
    SisterEvent { sister: String, event: String },
    /// A pattern was detected in recent activity
    PatternDetected { pattern: String },
    /// Time-based reminder
    Reminder { message: String },
}

/// Specification for what to watch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchSpec {
    pub id: String,
    pub trigger: WatchTriggerType,
    pub description: String,
    pub enabled: bool,
    pub cooldown_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchTriggerType {
    FileGlob { pattern: String },
    Interval { seconds: u64 },
    SisterTool { sister: String, tool: String },
}

/// A proactive update to push to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveUpdate {
    pub trigger: ProactiveTrigger,
    pub message: String,
    pub priority: UpdatePriority,
    pub actionable: bool,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePriority {
    Low,
    Medium,
    High,
}

/// Engine that monitors watches and generates proactive updates
pub struct ProactiveEngine {
    watches: parking_lot::Mutex<Vec<WatchSpec>>,
    pending_updates: parking_lot::Mutex<Vec<ProactiveUpdate>>,
    enabled: parking_lot::Mutex<bool>,
}

impl ProactiveEngine {
    pub fn new() -> Self {
        Self {
            watches: parking_lot::Mutex::new(Vec::new()),
            pending_updates: parking_lot::Mutex::new(Vec::new()),
            enabled: parking_lot::Mutex::new(true),
        }
    }

    /// Add a watch specification
    pub fn add_watch(&self, spec: WatchSpec) {
        self.watches.lock().push(spec);
    }

    /// Remove a watch by ID
    pub fn remove_watch(&self, id: &str) -> bool {
        let mut watches = self.watches.lock();
        let len_before = watches.len();
        watches.retain(|w| w.id != id);
        watches.len() < len_before
    }

    /// Get all active watches
    pub fn watches(&self) -> Vec<WatchSpec> {
        self.watches.lock().clone()
    }

    /// Process a trigger and generate an update if applicable
    pub fn process_trigger(&self, trigger: ProactiveTrigger) -> Option<ProactiveUpdate> {
        if !*self.enabled.lock() {
            return None;
        }

        let update = match &trigger {
            ProactiveTrigger::FileChanged { path } => {
                // Check if any watch matches this file
                let watches = self.watches.lock();
                let matched = watches.iter().any(|w| {
                    if let WatchTriggerType::FileGlob { pattern } = &w.trigger {
                        path.contains(pattern.trim_matches('*'))
                    } else {
                        false
                    }
                });
                if matched {
                    Some(ProactiveUpdate {
                        trigger: trigger.clone(),
                        message: format!("File changed: {}", path),
                        priority: UpdatePriority::Medium,
                        actionable: true,
                        suggested_action: Some("Review changes".into()),
                    })
                } else {
                    None
                }
            }
            ProactiveTrigger::SisterEvent { sister, event } => Some(ProactiveUpdate {
                trigger: trigger.clone(),
                message: format!("{} reported: {}", sister, event),
                priority: UpdatePriority::Medium,
                actionable: false,
                suggested_action: None,
            }),
            ProactiveTrigger::PatternDetected { pattern } => Some(ProactiveUpdate {
                trigger: trigger.clone(),
                message: format!("Pattern detected: {}", pattern),
                priority: UpdatePriority::Low,
                actionable: true,
                suggested_action: Some(format!("Apply pattern: {}", pattern)),
            }),
            ProactiveTrigger::Reminder { message } => Some(ProactiveUpdate {
                trigger: trigger.clone(),
                message: message.clone(),
                priority: UpdatePriority::High,
                actionable: false,
                suggested_action: None,
            }),
            ProactiveTrigger::ScheduledCheck { name } => Some(ProactiveUpdate {
                trigger: trigger.clone(),
                message: format!("Scheduled check: {}", name),
                priority: UpdatePriority::Low,
                actionable: false,
                suggested_action: None,
            }),
        };

        if let Some(ref u) = update {
            self.pending_updates.lock().push(u.clone());
        }

        update
    }

    /// Drain all pending updates
    pub fn drain_updates(&self) -> Vec<ProactiveUpdate> {
        std::mem::take(&mut *self.pending_updates.lock())
    }

    /// Number of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending_updates.lock().len()
    }

    /// Enable or disable the engine
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock() = enabled;
    }

    /// Whether the engine is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock()
    }
}

impl Default for ProactiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proactive_trigger_file() {
        let engine = ProactiveEngine::new();
        engine.add_watch(WatchSpec {
            id: "src-watch".into(),
            trigger: WatchTriggerType::FileGlob {
                pattern: "*.rs".into(),
            },
            description: "Watch Rust files".into(),
            enabled: true,
            cooldown_secs: 10,
        });

        let update = engine.process_trigger(ProactiveTrigger::FileChanged {
            path: "src/main.rs".into(),
        });
        assert!(update.is_some());
        assert!(update.unwrap().actionable);
    }

    #[test]
    fn test_proactive_trigger_no_match() {
        let engine = ProactiveEngine::new();
        engine.add_watch(WatchSpec {
            id: "rs-watch".into(),
            trigger: WatchTriggerType::FileGlob {
                pattern: "*.rs".into(),
            },
            description: "Watch Rust files".into(),
            enabled: true,
            cooldown_secs: 10,
        });

        let update = engine.process_trigger(ProactiveTrigger::FileChanged {
            path: "docs/readme.md".into(),
        });
        assert!(update.is_none());
    }

    #[test]
    fn test_proactive_sister_event() {
        let engine = ProactiveEngine::new();
        let update = engine.process_trigger(ProactiveTrigger::SisterEvent {
            sister: "memory".into(),
            event: "New pattern crystallized".into(),
        });
        assert!(update.is_some());
        assert_eq!(update.unwrap().priority, UpdatePriority::Medium);
    }

    #[test]
    fn test_proactive_disabled() {
        let engine = ProactiveEngine::new();
        engine.set_enabled(false);
        let update = engine.process_trigger(ProactiveTrigger::Reminder {
            message: "test".into(),
        });
        assert!(update.is_none());
    }

    #[test]
    fn test_drain_updates() {
        let engine = ProactiveEngine::new();
        engine.process_trigger(ProactiveTrigger::Reminder {
            message: "a".into(),
        });
        engine.process_trigger(ProactiveTrigger::Reminder {
            message: "b".into(),
        });
        assert_eq!(engine.pending_count(), 2);
        let updates = engine.drain_updates();
        assert_eq!(updates.len(), 2);
        assert_eq!(engine.pending_count(), 0);
    }

    #[test]
    fn test_remove_watch() {
        let engine = ProactiveEngine::new();
        engine.add_watch(WatchSpec {
            id: "w1".into(),
            trigger: WatchTriggerType::Interval { seconds: 60 },
            description: "test".into(),
            enabled: true,
            cooldown_secs: 10,
        });
        assert_eq!(engine.watches().len(), 1);
        assert!(engine.remove_watch("w1"));
        assert_eq!(engine.watches().len(), 0);
    }
}
