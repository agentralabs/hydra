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
#[path = "trace_tests.rs"]
mod tests;
