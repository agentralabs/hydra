//! Integration tests for hydra-observability.

use hydra_observability::context::LogContext;
use hydra_observability::exporter::{ExportFormat, ExportTarget, LogExporter};
use hydra_observability::filter::{FilterConfig, LogFilter};
use hydra_observability::logger::{LogEntry, LogLevel, StructuredLogger};
use hydra_observability::metrics::MetricsCollector;
use hydra_observability::traces::{SpanStatus, TraceManager};

#[test]
fn test_structured_log_format() {
    let ctx = LogContext::new()
        .with_trace("trace-abc")
        .with_span("span-123")
        .with_component("cognitive_loop")
        .with_phase("think")
        .with_run("run-42");

    let entry = LogEntry::with_context(LogLevel::Info, "LLM call completed", &ctx)
        .with_duration(234)
        .with_tokens(145);

    let json = entry.to_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["level"], "info");
    assert_eq!(parsed["message"], "LLM call completed");
    assert_eq!(parsed["component"], "cognitive_loop");
    assert_eq!(parsed["phase"], "think");
    assert_eq!(parsed["run_id"], "run-42");
    assert_eq!(parsed["trace_id"], "trace-abc");
    assert_eq!(parsed["span_id"], "span-123");
    assert_eq!(parsed["duration_ms"], 234);
    assert_eq!(parsed["tokens"], 145);
    assert!(parsed["timestamp"].as_str().unwrap().contains("T"));
}

#[test]
fn test_log_levels() {
    let logger = StructuredLogger::new(LogLevel::Warn, 100);
    assert!(!logger.log_msg(LogLevel::Trace, "trace"));
    assert!(!logger.log_msg(LogLevel::Debug, "debug"));
    assert!(!logger.log_msg(LogLevel::Info, "info"));
    assert!(logger.log_msg(LogLevel::Warn, "warn"));
    assert!(logger.log_msg(LogLevel::Error, "error"));
    assert_eq!(logger.count(), 2);
}

#[test]
fn test_log_context_and_correlation() {
    let logger = StructuredLogger::default();
    let ctx = LogContext::new_trace()
        .with_component("server")
        .with_run("run-1");

    // Log parent
    logger.log_ctx(LogLevel::Info, "Request received", &ctx);

    // Log child
    let child_ctx = ctx.child().with_phase("perceive");
    logger.log_ctx(LogLevel::Info, "Perceive started", &child_ctx);

    let entries = logger.entries();
    assert_eq!(entries.len(), 2);
    // Same trace ID
    assert_eq!(entries[0].trace_id, entries[1].trace_id);
    // Different span IDs
    assert_ne!(entries[0].span_id, entries[1].span_id);
}

#[test]
fn test_metric_counter() {
    let m = MetricsCollector::new();
    m.register("hydra_runs_total", "Total cognitive runs");

    m.counter_inc("hydra_runs_total");
    m.counter_inc("hydra_runs_total");
    m.counter_inc("hydra_runs_total");
    assert_eq!(m.counter_get("hydra_runs_total"), 3.0);

    let prom = m.export_prometheus();
    assert!(prom.contains("hydra_runs_total 3"));
    assert!(prom.contains("# HELP hydra_runs_total Total cognitive runs"));
}

#[test]
fn test_metric_gauge() {
    let m = MetricsCollector::new();
    m.gauge_set("hydra_active_runs", 5.0);
    assert_eq!(m.gauge_get("hydra_active_runs"), 5.0);

    m.gauge_inc("hydra_active_runs", -2.0);
    assert_eq!(m.gauge_get("hydra_active_runs"), 3.0);
}

#[test]
fn test_metric_histogram() {
    let m = MetricsCollector::new();
    m.register("hydra_run_duration_seconds", "Run duration");

    m.histogram_observe("hydra_run_duration_seconds", 0.1);
    m.histogram_observe("hydra_run_duration_seconds", 0.5);
    m.histogram_observe("hydra_run_duration_seconds", 2.0);

    let summary = m.histogram_summary("hydra_run_duration_seconds").unwrap();
    assert_eq!(summary.count, 3);
    assert!((summary.sum - 2.6).abs() < 0.001);
    assert!((summary.min - 0.1).abs() < f64::EPSILON);
    assert!((summary.max - 2.0).abs() < f64::EPSILON);

    let prom = m.export_prometheus();
    assert!(prom.contains("hydra_run_duration_seconds_sum"));
    assert!(prom.contains("hydra_run_duration_seconds_count 3"));
}

#[test]
fn test_span_creation_and_nesting() {
    let tm = TraceManager::new();

    let root = tm.start_span("request");
    let perceive = tm.start_child_span(&root, "perceive");
    let think = tm.start_child_span(&root, "think");
    let llm_call = tm.start_child_span(&think, "llm_call");

    // All same trace
    assert_eq!(perceive.trace_id, root.trace_id);
    assert_eq!(think.trace_id, root.trace_id);
    assert_eq!(llm_call.trace_id, root.trace_id);

    // Correct parent chain
    assert!(root.parent_span_id.is_none());
    assert_eq!(perceive.parent_span_id, Some(root.span_id.clone()));
    assert_eq!(llm_call.parent_span_id, Some(think.span_id.clone()));

    // End spans
    tm.end_span(&llm_call.span_id, SpanStatus::Completed);
    tm.end_span(&think.span_id, SpanStatus::Completed);
    tm.end_span(&perceive.span_id, SpanStatus::Completed);
    tm.end_span(&root.span_id, SpanStatus::Completed);

    assert_eq!(tm.active_span_count(), 0);
    assert_eq!(tm.span_count(), 4);

    let trace = tm.get_trace(&root.trace_id);
    assert_eq!(trace.len(), 4);
}

#[test]
fn test_exporter_file() {
    let logger = StructuredLogger::default();
    logger.log_msg(LogLevel::Info, "test log 1");
    logger.log_msg(LogLevel::Warn, "test log 2");

    let exporter = LogExporter::new(ExportFormat::JsonLines);
    let result = exporter.export(
        &logger,
        &ExportTarget::File {
            path: "/tmp/hydra.log".into(),
        },
    );
    assert!(result.success);
    assert_eq!(result.entries_exported, 2);
    assert!(result.target.contains("/tmp/hydra.log"));
}

#[test]
fn test_exporter_stdout() {
    let logger = StructuredLogger::default();
    logger.log_msg(LogLevel::Info, "stdout test");

    let exporter = LogExporter::new(ExportFormat::Text);
    let result = exporter.export(&logger, &ExportTarget::Stdout);
    assert!(result.success);
    assert_eq!(result.entries_exported, 1);

    let buffer = exporter.get_buffer();
    assert!(!buffer.is_empty());
    assert!(buffer[0].contains("stdout test"));
}

#[test]
fn test_filter_sampling() {
    let filter = LogFilter::new(FilterConfig {
        min_level: LogLevel::Info,
        sample_rate: 0.5,
        ..Default::default()
    });

    let entries: Vec<LogEntry> = (0..20)
        .map(|i| LogEntry::new(LogLevel::Info, &format!("msg {}", i)))
        .collect();

    let filtered = filter.filter(&entries);
    // With 50% sampling, roughly half should pass
    assert!(filtered.len() < entries.len());
    assert!(!filtered.is_empty());
}

#[test]
fn test_full_observability_pipeline() {
    // 1. Create trace context
    let ctx = LogContext::new_trace()
        .with_component("cognitive_loop")
        .with_run("run-full-test");

    // 2. Create trace spans
    let tm = TraceManager::new();
    let root = tm.start_span("cognitive_loop");

    // 3. Log with context
    let logger = StructuredLogger::default();
    logger.log_ctx(LogLevel::Info, "Run started", &ctx);

    // 4. Track metrics
    let metrics = MetricsCollector::new();
    metrics.counter_inc("hydra_runs_total");
    metrics.gauge_set("hydra_active_runs", 1.0);
    metrics.histogram_observe("hydra_run_duration_seconds", 0.45);

    // 5. Child span for phase
    let phase_ctx = ctx.child().with_phase("perceive");
    let _phase_span = tm.start_child_span(&root, "perceive");
    logger.log_ctx(LogLevel::Info, "Perceive completed", &phase_ctx);

    // 6. Export
    let exporter = LogExporter::default();
    let result = exporter.export(&logger, &ExportTarget::Buffer);
    assert!(result.success);
    assert_eq!(result.entries_exported, 2);

    // 7. Verify metrics
    assert_eq!(metrics.counter_get("hydra_runs_total"), 1.0);
    let prom = metrics.export_prometheus();
    assert!(prom.contains("hydra_runs_total"));

    // 8. Verify trace
    assert_eq!(tm.span_count(), 2);
    tm.end_span(&root.span_id, SpanStatus::Completed);
}
