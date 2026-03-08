//! StructuredLogger — JSON structured logging with context.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::context::LogContext;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// A structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level,
            message: message.into(),
            component: None,
            run_id: None,
            phase: None,
            trace_id: None,
            span_id: None,
            duration_ms: None,
            tokens: None,
            extra: HashMap::new(),
        }
    }

    /// Create from a log context
    pub fn with_context(level: LogLevel, message: &str, ctx: &LogContext) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level,
            message: message.into(),
            component: ctx.component.clone(),
            run_id: ctx.run_id.clone(),
            phase: ctx.phase.clone(),
            trace_id: ctx.trace_id.clone(),
            span_id: ctx.span_id.clone(),
            duration_ms: None,
            tokens: None,
            extra: ctx.attributes.clone(),
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens = Some(tokens);
        self
    }

    pub fn with_extra(mut self, key: &str, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                "{{\"error\":\"serialization failed\",\"message\":\"{}\"}}",
                self.message
            )
        })
    }
}

/// Structured logger that collects entries
pub struct StructuredLogger {
    entries: parking_lot::RwLock<Vec<LogEntry>>,
    min_level: LogLevel,
    max_entries: usize,
}

impl StructuredLogger {
    pub fn new(min_level: LogLevel, max_entries: usize) -> Self {
        Self {
            entries: parking_lot::RwLock::new(Vec::new()),
            min_level,
            max_entries,
        }
    }

    /// Log an entry (only if level >= min_level)
    pub fn log(&self, entry: LogEntry) -> bool {
        if entry.level < self.min_level {
            return false;
        }

        let mut entries = self.entries.write();
        entries.push(entry);

        // Evict oldest if over limit
        while entries.len() > self.max_entries {
            entries.remove(0);
        }
        true
    }

    /// Log with level and message
    pub fn log_msg(&self, level: LogLevel, message: &str) -> bool {
        self.log(LogEntry::new(level, message))
    }

    /// Log with context
    pub fn log_ctx(&self, level: LogLevel, message: &str, ctx: &LogContext) -> bool {
        self.log(LogEntry::with_context(level, message, ctx))
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.read().clone()
    }

    /// Get entries by level
    pub fn entries_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.level == level)
            .cloned()
            .collect()
    }

    /// Get entries by component
    pub fn entries_by_component(&self, component: &str) -> Vec<LogEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.component.as_deref() == Some(component))
            .cloned()
            .collect()
    }

    /// Export all entries as JSON lines
    pub fn export_jsonl(&self) -> String {
        self.entries
            .read()
            .iter()
            .map(|e| e.to_json())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Entry count
    pub fn count(&self) -> usize {
        self.entries.read().len()
    }
}

impl Default for StructuredLogger {
    fn default() -> Self {
        Self::new(LogLevel::Info, 10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_log_format() {
        let entry = LogEntry::new(LogLevel::Info, "LLM call completed")
            .with_duration(234)
            .with_tokens(145)
            .with_extra("provider", serde_json::json!("anthropic"));

        let json = entry.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["level"], "info");
        assert_eq!(parsed["message"], "LLM call completed");
        assert_eq!(parsed["duration_ms"], 234);
        assert_eq!(parsed["tokens"], 145);
        assert!(parsed["timestamp"].as_str().is_some());
    }

    #[test]
    fn test_log_levels() {
        let logger = StructuredLogger::new(LogLevel::Warn, 100);

        assert!(!logger.log_msg(LogLevel::Debug, "debug msg"));
        assert!(!logger.log_msg(LogLevel::Info, "info msg"));
        assert!(logger.log_msg(LogLevel::Warn, "warn msg"));
        assert!(logger.log_msg(LogLevel::Error, "error msg"));
        assert_eq!(logger.count(), 2);
    }

    #[test]
    fn test_log_with_context() {
        let logger = StructuredLogger::default();
        let ctx = LogContext::new()
            .with_trace("trace-1")
            .with_component("cognitive_loop")
            .with_phase("think")
            .with_run("run-42");

        logger.log_ctx(LogLevel::Info, "Phase completed", &ctx);

        let entries = logger.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].trace_id.as_deref(), Some("trace-1"));
        assert_eq!(entries[0].component.as_deref(), Some("cognitive_loop"));
        assert_eq!(entries[0].phase.as_deref(), Some("think"));
        assert_eq!(entries[0].run_id.as_deref(), Some("run-42"));
    }
}
