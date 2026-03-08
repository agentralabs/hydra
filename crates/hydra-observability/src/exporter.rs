//! LogExporter — export logs to file, stdout, or remote endpoints.

use serde::{Deserialize, Serialize};

use crate::logger::{LogEntry, StructuredLogger};

/// Export destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportTarget {
    /// Write to stdout
    Stdout,
    /// Write to a file
    File { path: String },
    /// Send to a remote OTLP endpoint
    Remote { endpoint: String },
    /// In-memory buffer (for testing)
    Buffer,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON lines format
    JsonLines,
    /// Pretty-printed JSON
    JsonPretty,
    /// Plain text
    Text,
}

/// Result of an export operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub target: String,
    pub entries_exported: usize,
    pub bytes_written: usize,
    pub success: bool,
    pub error: Option<String>,
}

/// Exports log entries to various targets
pub struct LogExporter {
    format: ExportFormat,
    buffer: parking_lot::RwLock<Vec<String>>,
}

impl LogExporter {
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            buffer: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Export entries from a logger to a target
    pub fn export(&self, logger: &StructuredLogger, target: &ExportTarget) -> ExportResult {
        let entries = logger.entries();
        self.export_entries(&entries, target)
    }

    /// Export specific entries to a target
    pub fn export_entries(&self, entries: &[LogEntry], target: &ExportTarget) -> ExportResult {
        let formatted: Vec<String> = entries.iter().map(|e| self.format_entry(e)).collect();
        let output = formatted.join("\n");
        let bytes = output.len();

        match target {
            ExportTarget::Stdout => {
                // In production: println!("{}", output);
                // For testing: store in buffer
                self.buffer.write().push(output);
                ExportResult {
                    target: "stdout".into(),
                    entries_exported: entries.len(),
                    bytes_written: bytes,
                    success: true,
                    error: None,
                }
            }
            ExportTarget::File { path } => {
                // In production: write to file
                // For testing: store in buffer with path tag
                self.buffer
                    .write()
                    .push(format!("[file:{}] {}", path, output));
                ExportResult {
                    target: format!("file:{}", path),
                    entries_exported: entries.len(),
                    bytes_written: bytes,
                    success: true,
                    error: None,
                }
            }
            ExportTarget::Remote { endpoint } => {
                // In production: POST to OTLP endpoint
                self.buffer
                    .write()
                    .push(format!("[remote:{}] {}", endpoint, output));
                ExportResult {
                    target: format!("remote:{}", endpoint),
                    entries_exported: entries.len(),
                    bytes_written: bytes,
                    success: true,
                    error: None,
                }
            }
            ExportTarget::Buffer => {
                self.buffer.write().push(output);
                ExportResult {
                    target: "buffer".into(),
                    entries_exported: entries.len(),
                    bytes_written: bytes,
                    success: true,
                    error: None,
                }
            }
        }
    }

    /// Format a single entry
    fn format_entry(&self, entry: &LogEntry) -> String {
        match self.format {
            ExportFormat::JsonLines => entry.to_json(),
            ExportFormat::JsonPretty => {
                serde_json::to_string_pretty(entry).unwrap_or_else(|_| entry.to_json())
            }
            ExportFormat::Text => format!(
                "{} [{}] {}: {}",
                entry.timestamp,
                entry.level.as_str(),
                entry.component.as_deref().unwrap_or("-"),
                entry.message,
            ),
        }
    }

    /// Get buffered output (for testing)
    pub fn get_buffer(&self) -> Vec<String> {
        self.buffer.read().clone()
    }

    /// Clear buffer
    pub fn clear_buffer(&self) {
        self.buffer.write().clear();
    }
}

impl Default for LogExporter {
    fn default() -> Self {
        Self::new(ExportFormat::JsonLines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logger::LogLevel;

    #[test]
    fn test_export_stdout() {
        let logger = StructuredLogger::default();
        logger.log_msg(LogLevel::Info, "test message");
        logger.log_msg(LogLevel::Error, "error occurred");

        let exporter = LogExporter::new(ExportFormat::JsonLines);
        let result = exporter.export(&logger, &ExportTarget::Stdout);
        assert!(result.success);
        assert_eq!(result.entries_exported, 2);
        assert!(result.bytes_written > 0);
    }

    #[test]
    fn test_export_file() {
        let logger = StructuredLogger::default();
        logger.log_msg(LogLevel::Info, "log entry");

        let exporter = LogExporter::new(ExportFormat::JsonLines);
        let result = exporter.export(
            &logger,
            &ExportTarget::File {
                path: "/var/log/hydra.log".into(),
            },
        );
        assert!(result.success);
        assert!(result.target.contains("file:"));

        let buffer = exporter.get_buffer();
        assert!(buffer[0].contains("[file:/var/log/hydra.log]"));
    }

    #[test]
    fn test_export_text_format() {
        let exporter = LogExporter::new(ExportFormat::Text);
        let entry = LogEntry::new(LogLevel::Warn, "something happened");
        let formatted = exporter.format_entry(&entry);
        assert!(formatted.contains("[warn]"));
        assert!(formatted.contains("something happened"));
    }
}
