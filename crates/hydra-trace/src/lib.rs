// hydra-trace: Distributed tracing for cognitive loops

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A trace span representing a unit of work
#[derive(Debug, Clone)]
pub struct TraceSpan {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub start_time: String,
    pub duration_ms: u64,
    pub status: SpanStatus,
    pub attributes: HashMap<String, String>,
    pub events: Vec<SpanEvent>,
}

/// Status of a trace span
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    Ok,
    Error,
    Timeout,
    Cancelled,
}

impl SpanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn is_error(&self) -> bool {
        !self.is_success()
    }
}

/// An event within a span
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: String,
    pub attributes: HashMap<String, String>,
}

/// A complete trace (collection of spans)
#[derive(Debug, Clone)]
pub struct Trace {
    pub id: String,
    pub spans: Vec<TraceSpan>,
    pub created_at: String,
}

impl Trace {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            spans: Vec::new(),
            created_at: "now".into(),
        }
    }

    pub fn add_span(&mut self, span: TraceSpan) {
        self.spans.push(span);
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    pub fn root_spans(&self) -> Vec<&TraceSpan> {
        self.spans.iter().filter(|s| s.parent_id.is_none()).collect()
    }

    pub fn child_spans(&self, parent_id: &str) -> Vec<&TraceSpan> {
        self.spans
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.spans.iter().map(|s| s.duration_ms).sum()
    }

    pub fn has_errors(&self) -> bool {
        self.spans.iter().any(|s| s.status.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.spans.iter().filter(|s| s.status.is_error()).count()
    }
}

/// Active span builder for in-progress tracing
pub struct SpanBuilder {
    id: String,
    parent_id: Option<String>,
    name: String,
    attributes: HashMap<String, String>,
    events: Vec<SpanEvent>,
    start: Instant,
}

impl SpanBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            parent_id: None,
            name: name.into(),
            attributes: HashMap::new(),
            events: Vec::new(),
            start: Instant::now(),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: "now".into(),
            attributes: HashMap::new(),
        });
    }

    pub fn finish(self, status: SpanStatus) -> TraceSpan {
        TraceSpan {
            id: self.id,
            parent_id: self.parent_id,
            name: self.name,
            start_time: "now".into(),
            duration_ms: self.start.elapsed().as_millis() as u64,
            status,
            attributes: self.attributes,
            events: self.events,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Trace collector
pub struct TraceCollector {
    traces: HashMap<String, Trace>,
    max_traces: usize,
}

impl TraceCollector {
    pub fn new(max_traces: usize) -> Self {
        Self {
            traces: HashMap::new(),
            max_traces,
        }
    }

    pub fn start_trace(&mut self, id: &str) {
        if self.traces.len() >= self.max_traces {
            // Remove oldest (arbitrary key for now)
            if let Some(oldest) = self.traces.keys().next().cloned() {
                self.traces.remove(&oldest);
            }
        }
        self.traces.insert(id.into(), Trace::new(id));
    }

    pub fn add_span(&mut self, trace_id: &str, span: TraceSpan) {
        if let Some(trace) = self.traces.get_mut(trace_id) {
            trace.add_span(span);
        }
    }

    pub fn get_trace(&self, id: &str) -> Option<&Trace> {
        self.traces.get(id)
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    pub fn all_traces(&self) -> Vec<&Trace> {
        self.traces.values().collect()
    }
}

impl Default for TraceCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpanStatus tests ───────────────────────────────────

    #[test]
    fn test_span_status_as_str() {
        assert_eq!(SpanStatus::Ok.as_str(), "ok");
        assert_eq!(SpanStatus::Error.as_str(), "error");
        assert_eq!(SpanStatus::Timeout.as_str(), "timeout");
        assert_eq!(SpanStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_span_status_is_success() {
        assert!(SpanStatus::Ok.is_success());
        assert!(!SpanStatus::Error.is_success());
        assert!(!SpanStatus::Timeout.is_success());
        assert!(!SpanStatus::Cancelled.is_success());
    }

    #[test]
    fn test_span_status_is_error() {
        assert!(!SpanStatus::Ok.is_error());
        assert!(SpanStatus::Error.is_error());
        assert!(SpanStatus::Timeout.is_error());
        assert!(SpanStatus::Cancelled.is_error());
    }

    // ── Trace tests ────────────────────────────────────────

    #[test]
    fn test_trace_new() {
        let trace = Trace::new("trace-1");
        assert_eq!(trace.id, "trace-1");
        assert_eq!(trace.span_count(), 0);
    }

    #[test]
    fn test_trace_add_span() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "s1".into(),
            parent_id: None,
            name: "root".into(),
            start_time: "now".into(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert_eq!(trace.span_count(), 1);
    }

    #[test]
    fn test_trace_root_spans() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "root".into(),
            parent_id: None,
            name: "root".into(),
            start_time: "now".into(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "child".into(),
            parent_id: Some("root".into()),
            name: "child".into(),
            start_time: "now".into(),
            duration_ms: 50,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert_eq!(trace.root_spans().len(), 1);
        assert_eq!(trace.root_spans()[0].id, "root");
    }

    #[test]
    fn test_trace_child_spans() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "root".into(),
            parent_id: None,
            name: "root".into(),
            start_time: "now".into(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "c1".into(),
            parent_id: Some("root".into()),
            name: "child1".into(),
            start_time: "now".into(),
            duration_ms: 30,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "c2".into(),
            parent_id: Some("root".into()),
            name: "child2".into(),
            start_time: "now".into(),
            duration_ms: 20,
            status: SpanStatus::Error,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert_eq!(trace.child_spans("root").len(), 2);
        assert_eq!(trace.child_spans("c1").len(), 0);
    }

    #[test]
    fn test_trace_total_duration() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "s1".into(),
            parent_id: None,
            name: "a".into(),
            start_time: "now".into(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "s2".into(),
            parent_id: None,
            name: "b".into(),
            start_time: "now".into(),
            duration_ms: 200,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert_eq!(trace.total_duration_ms(), 300);
    }

    #[test]
    fn test_trace_has_errors() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "s1".into(),
            parent_id: None,
            name: "ok".into(),
            start_time: "now".into(),
            duration_ms: 10,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert!(!trace.has_errors());

        trace.add_span(TraceSpan {
            id: "s2".into(),
            parent_id: None,
            name: "fail".into(),
            start_time: "now".into(),
            duration_ms: 10,
            status: SpanStatus::Error,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert!(trace.has_errors());
    }

    #[test]
    fn test_trace_error_count() {
        let mut trace = Trace::new("t1");
        trace.add_span(TraceSpan {
            id: "s1".into(),
            parent_id: None,
            name: "ok".into(),
            start_time: "now".into(),
            duration_ms: 10,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "s2".into(),
            parent_id: None,
            name: "err".into(),
            start_time: "now".into(),
            duration_ms: 10,
            status: SpanStatus::Error,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        trace.add_span(TraceSpan {
            id: "s3".into(),
            parent_id: None,
            name: "timeout".into(),
            start_time: "now".into(),
            duration_ms: 10,
            status: SpanStatus::Timeout,
            attributes: HashMap::new(),
            events: Vec::new(),
        });
        assert_eq!(trace.error_count(), 2);
    }

    // ── SpanBuilder tests ──────────────────────────────────

    #[test]
    fn test_span_builder_basic() {
        let builder = SpanBuilder::new("s1", "test-span");
        let span = builder.finish(SpanStatus::Ok);
        assert_eq!(span.id, "s1");
        assert_eq!(span.name, "test-span");
        assert_eq!(span.status, SpanStatus::Ok);
        assert!(span.parent_id.is_none());
    }

    #[test]
    fn test_span_builder_with_parent() {
        let builder = SpanBuilder::new("child", "child-span").with_parent("parent");
        let span = builder.finish(SpanStatus::Ok);
        assert_eq!(span.parent_id, Some("parent".into()));
    }

    #[test]
    fn test_span_builder_attributes() {
        let mut builder = SpanBuilder::new("s1", "span");
        builder.set_attribute("key", "value");
        builder.set_attribute("phase", "perceive");
        let span = builder.finish(SpanStatus::Ok);
        assert_eq!(span.attributes.get("key").unwrap(), "value");
        assert_eq!(span.attributes.get("phase").unwrap(), "perceive");
    }

    #[test]
    fn test_span_builder_events() {
        let mut builder = SpanBuilder::new("s1", "span");
        builder.add_event("started");
        builder.add_event("checkpoint");
        let span = builder.finish(SpanStatus::Ok);
        assert_eq!(span.events.len(), 2);
        assert_eq!(span.events[0].name, "started");
    }

    #[test]
    fn test_span_builder_elapsed() {
        let builder = SpanBuilder::new("s1", "span");
        std::thread::sleep(Duration::from_millis(1));
        assert!(builder.elapsed() >= Duration::from_millis(1));
    }

    // ── TraceCollector tests ───────────────────────────────

    #[test]
    fn test_collector_new() {
        let collector = TraceCollector::new(100);
        assert_eq!(collector.trace_count(), 0);
    }

    #[test]
    fn test_collector_default() {
        let collector = TraceCollector::default();
        assert_eq!(collector.trace_count(), 0);
    }

    #[test]
    fn test_collector_start_trace() {
        let mut collector = TraceCollector::new(100);
        collector.start_trace("t1");
        assert_eq!(collector.trace_count(), 1);
        assert!(collector.get_trace("t1").is_some());
    }

    #[test]
    fn test_collector_add_span() {
        let mut collector = TraceCollector::new(100);
        collector.start_trace("t1");
        collector.add_span(
            "t1",
            TraceSpan {
                id: "s1".into(),
                parent_id: None,
                name: "root".into(),
                start_time: "now".into(),
                duration_ms: 50,
                status: SpanStatus::Ok,
                attributes: HashMap::new(),
                events: Vec::new(),
            },
        );
        assert_eq!(collector.get_trace("t1").unwrap().span_count(), 1);
    }

    #[test]
    fn test_collector_max_traces() {
        let mut collector = TraceCollector::new(2);
        collector.start_trace("t1");
        collector.start_trace("t2");
        collector.start_trace("t3"); // Should evict one
        assert_eq!(collector.trace_count(), 2);
    }

    #[test]
    fn test_collector_get_nonexistent() {
        let collector = TraceCollector::new(100);
        assert!(collector.get_trace("none").is_none());
    }

    #[test]
    fn test_collector_all_traces() {
        let mut collector = TraceCollector::new(100);
        collector.start_trace("t1");
        collector.start_trace("t2");
        assert_eq!(collector.all_traces().len(), 2);
    }

    #[test]
    fn test_collector_add_span_nonexistent_trace() {
        let mut collector = TraceCollector::new(100);
        // Should not panic, just no-op
        collector.add_span(
            "nonexistent",
            TraceSpan {
                id: "s1".into(),
                parent_id: None,
                name: "orphan".into(),
                start_time: "now".into(),
                duration_ms: 10,
                status: SpanStatus::Ok,
                attributes: HashMap::new(),
                events: Vec::new(),
            },
        );
        assert_eq!(collector.trace_count(), 0);
    }
}
