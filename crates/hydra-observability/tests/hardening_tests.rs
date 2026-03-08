//! Category 1: Unit Gap Fill — hydra-observability edge cases.

use hydra_observability::*;

// === Log rotation (simulated via clear) ===

#[test]
fn test_log_clear_after_export() {
    let logger = logger::StructuredLogger::new(logger::LogLevel::Info, 10000);
    for i in 0..100 {
        logger.log_msg(logger::LogLevel::Info, &format!("msg {}", i));
    }
    assert_eq!(logger.count(), 100);
    let _export = logger.export_jsonl();
    logger.clear();
    assert_eq!(logger.count(), 0);
}

// === Metric overflow ===

#[test]
fn test_counter_large_values() {
    let collector = metrics::MetricsCollector::new();
    collector.register("big_counter", "A big counter");
    for _ in 0..10_000 {
        collector.counter_inc("big_counter");
    }
    assert_eq!(collector.counter_get("big_counter"), 10_000.0);
}

#[test]
fn test_histogram_many_observations() {
    let collector = metrics::MetricsCollector::new();
    collector.register("latency", "Latency");
    for i in 0..1000 {
        collector.histogram_observe("latency", i as f64);
    }
    let summary = collector.histogram_summary("latency").unwrap();
    assert_eq!(summary.count, 1000);
    assert_eq!(summary.min, 0.0);
    assert_eq!(summary.max, 999.0);
}

#[test]
fn test_gauge_set_and_read() {
    let collector = metrics::MetricsCollector::new();
    collector.register("memory", "Memory");
    collector.gauge_set("memory", 42.0);
    assert_eq!(collector.gauge_get("memory"), 42.0);
    collector.gauge_set("memory", 100.0);
    assert_eq!(collector.gauge_get("memory"), 100.0);
}

// === Trace orphan spans ===

#[test]
fn test_trace_orphan_span() {
    let mgr = traces::TraceManager::new();
    let span = mgr.start_span("orphan_span");
    // Don't end it — it's orphaned
    assert_eq!(mgr.active_span_count(), 1);
    // End it
    mgr.end_span(&span.span_id, traces::SpanStatus::Completed);
    assert_eq!(mgr.active_span_count(), 0);
}

#[test]
fn test_trace_parent_child() {
    let mgr = traces::TraceManager::new();
    let parent = mgr.start_span("parent");
    let child = mgr.start_child_span(&parent, "child");
    assert_eq!(
        child.parent_span_id.as_deref(),
        Some(parent.span_id.as_str())
    );
    assert_eq!(child.trace_id, parent.trace_id);

    mgr.end_span(&child.span_id, traces::SpanStatus::Completed);
    mgr.end_span(&parent.span_id, traces::SpanStatus::Completed);
    assert_eq!(mgr.active_span_count(), 0);
}

#[test]
fn test_trace_add_event() {
    let mgr = traces::TraceManager::new();
    let span = mgr.start_span("test");
    mgr.add_event(
        &span.span_id,
        "checkpoint",
        std::collections::HashMap::new(),
    );
    let s = mgr.get_span(&span.span_id).unwrap();
    assert_eq!(s.events.len(), 1);
}

// === Filter ===

#[test]
fn test_filter_level() {
    let filter = filter::LogFilter::new(filter::FilterConfig {
        min_level: logger::LogLevel::Warn,
        component_levels: Vec::new(),
        sample_rate: 1.0,
        always_pass: vec![],
    });
    let debug_entry = logger::LogEntry::new(logger::LogLevel::Debug, "any");
    let info_entry = logger::LogEntry::new(logger::LogLevel::Info, "any");
    let warn_entry = logger::LogEntry::new(logger::LogLevel::Warn, "any");
    let error_entry = logger::LogEntry::new(logger::LogLevel::Error, "any");
    assert!(!filter.should_pass(&debug_entry));
    assert!(!filter.should_pass(&info_entry));
    assert!(filter.should_pass(&warn_entry));
    assert!(filter.should_pass(&error_entry));
}

#[test]
fn test_filter_component_override() {
    let component_levels = vec![("noisy".into(), logger::LogLevel::Error)];
    let filter = filter::LogFilter::new(filter::FilterConfig {
        min_level: logger::LogLevel::Debug,
        component_levels,
        sample_rate: 1.0,
        always_pass: vec![],
    });

    let mut normal_debug = logger::LogEntry::new(logger::LogLevel::Debug, "normal");
    normal_debug.component = Some("normal".into());
    assert!(filter.should_pass(&normal_debug));

    let mut noisy_warn = logger::LogEntry::new(logger::LogLevel::Warn, "noisy");
    noisy_warn.component = Some("noisy".into());
    assert!(!filter.should_pass(&noisy_warn)); // overridden to Error only

    let mut noisy_error = logger::LogEntry::new(logger::LogLevel::Error, "noisy");
    noisy_error.component = Some("noisy".into());
    assert!(filter.should_pass(&noisy_error));
}

// === Exporter ===

#[test]
fn test_exporter_buffer() {
    let exporter = exporter::LogExporter::new(exporter::ExportFormat::JsonLines);
    let entries = vec![logger::LogEntry::new(
        logger::LogLevel::Info,
        "test message",
    )];
    let result = exporter.export_entries(&entries, &exporter::ExportTarget::Buffer);
    assert!(result.success);
    assert!(result.entries_exported == 1);
    let buf = exporter.get_buffer();
    assert!(!buf.is_empty());
}

// === Context ===

#[test]
fn test_context_child_inherits() {
    let parent = context::LogContext::new()
        .with_trace("trace-1")
        .with_span("span-parent")
        .with_component("parent")
        .with_run("run-1");
    let child = parent.child();
    assert_eq!(child.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(child.run_id.as_deref(), Some("run-1"));
    assert_eq!(child.parent_span_id.as_deref(), Some("span-parent"));
}
