//! LogFilter — level-based filtering and sampling.

use serde::{Deserialize, Serialize};

use crate::logger::{LogEntry, LogLevel};

/// Filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Minimum log level to pass through
    pub min_level: LogLevel,
    /// Component-specific level overrides
    pub component_levels: Vec<(String, LogLevel)>,
    /// Sampling rate (0.0 = drop all, 1.0 = keep all)
    pub sample_rate: f64,
    /// Always pass entries matching these components (bypass sampling)
    pub always_pass: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            component_levels: Vec::new(),
            sample_rate: 1.0,
            always_pass: vec!["error".into()],
        }
    }
}

/// Log filter with level checking and sampling
pub struct LogFilter {
    config: FilterConfig,
    sample_counter: parking_lot::Mutex<u64>,
}

impl LogFilter {
    pub fn new(config: FilterConfig) -> Self {
        Self {
            config,
            sample_counter: parking_lot::Mutex::new(0),
        }
    }

    /// Check if a log entry should pass the filter
    pub fn should_pass(&self, entry: &LogEntry) -> bool {
        // Error level always passes
        if entry.level == LogLevel::Error {
            return true;
        }

        // Check component-specific level
        if let Some(ref component) = entry.component {
            // Always-pass components bypass all filtering
            if self.config.always_pass.contains(component) {
                return true;
            }

            // Component-specific level override
            for (comp, level) in &self.config.component_levels {
                if comp == component {
                    if entry.level < *level {
                        return false;
                    }
                    // Passed component check, continue to sampling
                    return self.sample_check();
                }
            }
        }

        // Global level check
        if entry.level < self.config.min_level {
            return false;
        }

        // Sampling
        self.sample_check()
    }

    /// Apply sampling
    fn sample_check(&self) -> bool {
        if self.config.sample_rate >= 1.0 {
            return true;
        }
        if self.config.sample_rate <= 0.0 {
            return false;
        }

        let mut counter = self.sample_counter.lock();
        *counter += 1;
        let threshold = (1.0 / self.config.sample_rate) as u64;
        *counter % threshold == 0
    }

    /// Filter a batch of entries
    pub fn filter(&self, entries: &[LogEntry]) -> Vec<LogEntry> {
        entries
            .iter()
            .filter(|e| self.should_pass(e))
            .cloned()
            .collect()
    }

    /// Get current config
    pub fn config(&self) -> &FilterConfig {
        &self.config
    }

    /// Update minimum level
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.config.min_level = level;
    }

    /// Update sample rate
    pub fn set_sample_rate(&mut self, rate: f64) {
        self.config.sample_rate = rate.clamp(0.0, 1.0);
    }
}

impl Default for LogFilter {
    fn default() -> Self {
        Self::new(FilterConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_filtering() {
        let filter = LogFilter::new(FilterConfig {
            min_level: LogLevel::Warn,
            ..Default::default()
        });

        let debug_entry = LogEntry::new(LogLevel::Debug, "debug");
        let info_entry = LogEntry::new(LogLevel::Info, "info");
        let warn_entry = LogEntry::new(LogLevel::Warn, "warn");
        let error_entry = LogEntry::new(LogLevel::Error, "error");

        assert!(!filter.should_pass(&debug_entry));
        assert!(!filter.should_pass(&info_entry));
        assert!(filter.should_pass(&warn_entry));
        assert!(filter.should_pass(&error_entry)); // errors always pass
    }

    #[test]
    fn test_component_level_override() {
        let filter = LogFilter::new(FilterConfig {
            min_level: LogLevel::Info,
            component_levels: vec![("noisy_module".into(), LogLevel::Error)],
            ..Default::default()
        });

        let mut entry = LogEntry::new(LogLevel::Info, "from noisy module");
        entry.component = Some("noisy_module".into());
        assert!(!filter.should_pass(&entry));

        let mut entry2 = LogEntry::new(LogLevel::Info, "from quiet module");
        entry2.component = Some("quiet_module".into());
        assert!(filter.should_pass(&entry2));
    }

    #[test]
    fn test_sampling() {
        let filter = LogFilter::new(FilterConfig {
            min_level: LogLevel::Info,
            sample_rate: 0.5,
            ..Default::default()
        });

        let entries: Vec<LogEntry> = (0..10)
            .map(|i| LogEntry::new(LogLevel::Info, &format!("msg {}", i)))
            .collect();

        let filtered = filter.filter(&entries);
        assert!(filtered.len() < entries.len());
        assert!(!filtered.is_empty());
    }
}
