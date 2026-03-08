use super::types::{AlertSeverity, ProactiveUpdate};

/// Engine that manages proactive update delivery to the user.
///
/// Tracks update history, enforces silence limits, and provides
/// convenience methods for common update patterns.
pub struct ProactiveEngine {
    updates: Vec<ProactiveUpdate>,
    max_silence_ms: u64,
    progress_interval_ms: u64,
    last_update_ms: u64,
}

impl ProactiveEngine {
    /// Create a new engine with default settings (5s silence limit, 3s progress interval)
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            max_silence_ms: 5000,
            progress_interval_ms: 3000,
            last_update_ms: 0,
        }
    }

    /// Create a new engine with custom timing configuration
    pub fn with_config(max_silence_ms: u64, progress_interval_ms: u64) -> Self {
        Self {
            updates: Vec::new(),
            max_silence_ms,
            progress_interval_ms,
            last_update_ms: 0,
        }
    }

    /// Send an update, recording the current timestamp
    pub fn send(&mut self, update: ProactiveUpdate) {
        self.last_update_ms = current_time_ms();
        self.updates.push(update);
    }

    /// Check whether the engine has been silent too long
    pub fn check_silence(&self, now_ms: u64) -> bool {
        if self.last_update_ms == 0 {
            return false;
        }
        now_ms.saturating_sub(self.last_update_ms) > self.max_silence_ms
    }

    /// Return the most recent update, if any
    pub fn last_update(&self) -> Option<&ProactiveUpdate> {
        self.updates.last()
    }

    /// Drain all pending updates, returning them and clearing the internal buffer
    pub fn drain(&mut self) -> Vec<ProactiveUpdate> {
        std::mem::take(&mut self.updates)
    }

    /// Convenience: send an acknowledgment update
    pub fn acknowledge(&mut self, message: &str) {
        self.send(ProactiveUpdate::Acknowledgment {
            message: message.to_string(),
            estimated_duration: None,
        });
    }

    /// Convenience: send a progress update
    pub fn progress(&mut self, percent: f32, step: &str, remaining: usize) {
        self.send(ProactiveUpdate::Progress {
            percent,
            current_step: step.to_string(),
            steps_remaining: remaining,
        });
    }

    /// Convenience: send a completion update
    pub fn complete(&mut self, summary: &str, changes: Vec<String>, next_steps: Vec<String>) {
        self.send(ProactiveUpdate::Completion {
            summary: summary.to_string(),
            changes,
            next_steps,
        });
    }

    /// Convenience: send an alert
    pub fn alert(&mut self, severity: AlertSeverity, message: &str, recoverable: bool) {
        self.send(ProactiveUpdate::Alert {
            severity,
            message: message.to_string(),
            recoverable,
            action_required: None,
        });
    }

    /// Get the configured max silence duration in milliseconds
    pub fn max_silence_ms(&self) -> u64 {
        self.max_silence_ms
    }

    /// Get the configured progress interval in milliseconds
    pub fn progress_interval_ms(&self) -> u64 {
        self.progress_interval_ms
    }

    /// Get the total number of updates sent (including drained)
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }
}

impl Default for ProactiveEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let engine = ProactiveEngine::new();
        assert_eq!(engine.max_silence_ms(), 5000);
        assert_eq!(engine.progress_interval_ms(), 3000);
        assert_eq!(engine.update_count(), 0);
    }

    #[test]
    fn test_with_config() {
        let engine = ProactiveEngine::with_config(10000, 5000);
        assert_eq!(engine.max_silence_ms(), 10000);
        assert_eq!(engine.progress_interval_ms(), 5000);
    }

    #[test]
    fn test_acknowledge() {
        let mut engine = ProactiveEngine::new();
        engine.acknowledge("Got it");
        assert_eq!(engine.update_count(), 1);
        let update = engine.last_update().unwrap();
        assert!(matches!(update, ProactiveUpdate::Acknowledgment { .. }));
    }

    #[test]
    fn test_progress() {
        let mut engine = ProactiveEngine::new();
        engine.progress(50.0, "Building", 3);
        let update = engine.last_update().unwrap();
        if let ProactiveUpdate::Progress { percent, current_step, steps_remaining } = update {
            assert_eq!(*percent, 50.0);
            assert_eq!(current_step, "Building");
            assert_eq!(*steps_remaining, 3);
        } else {
            panic!("Expected Progress update");
        }
    }

    #[test]
    fn test_complete() {
        let mut engine = ProactiveEngine::new();
        engine.complete("All done", vec!["changed A".into()], vec!["run tests".into()]);
        let update = engine.last_update().unwrap();
        assert!(matches!(update, ProactiveUpdate::Completion { .. }));
    }

    #[test]
    fn test_alert() {
        let mut engine = ProactiveEngine::new();
        engine.alert(AlertSeverity::Warning, "Low disk", true);
        let update = engine.last_update().unwrap();
        if let ProactiveUpdate::Alert { severity, recoverable, .. } = update {
            assert_eq!(*severity, AlertSeverity::Warning);
            assert!(*recoverable);
        } else {
            panic!("Expected Alert update");
        }
    }

    #[test]
    fn test_drain() {
        let mut engine = ProactiveEngine::new();
        engine.acknowledge("a");
        engine.acknowledge("b");
        assert_eq!(engine.update_count(), 2);
        let updates = engine.drain();
        assert_eq!(updates.len(), 2);
        assert_eq!(engine.update_count(), 0);
    }

    #[test]
    fn test_check_silence_no_updates() {
        let engine = ProactiveEngine::new();
        assert!(!engine.check_silence(current_time_ms()));
    }

    #[test]
    fn test_check_silence_after_update() {
        let mut engine = ProactiveEngine::new();
        engine.acknowledge("test");
        // Immediately after should not be silent
        assert!(!engine.check_silence(current_time_ms()));
        // Far future should be silent
        assert!(engine.check_silence(current_time_ms() + 10000));
    }

    #[test]
    fn test_default() {
        let engine = ProactiveEngine::default();
        assert_eq!(engine.max_silence_ms(), 5000);
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Error);
        assert!(AlertSeverity::Error < AlertSeverity::Critical);
    }
}
