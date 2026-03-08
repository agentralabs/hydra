//! TraceManager — distributed tracing with spans and context propagation.

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Active,
    Completed,
    Error,
}

/// A trace span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub status: SpanStatus,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_ms: Option<u64>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
}

/// An event within a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Manages distributed traces
pub struct TraceManager {
    spans: RwLock<HashMap<String, Span>>,
    traces: RwLock<HashMap<String, Vec<String>>>, // trace_id -> [span_ids]
}

impl TraceManager {
    pub fn new() -> Self {
        Self {
            spans: RwLock::new(HashMap::new()),
            traces: RwLock::new(HashMap::new()),
        }
    }

    /// Start a new root span (creates a new trace)
    pub fn start_span(&self, name: &str) -> Span {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let span_id = short_id();

        let span = Span {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            parent_span_id: None,
            name: name.into(),
            status: SpanStatus::Active,
            start_time: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            end_time: None,
            duration_ms: None,
            attributes: HashMap::new(),
            events: Vec::new(),
        };

        self.spans.write().insert(span_id.clone(), span.clone());
        self.traces
            .write()
            .entry(trace_id)
            .or_default()
            .push(span_id);

        span
    }

    /// Start a child span under a parent
    pub fn start_child_span(&self, parent: &Span, name: &str) -> Span {
        let span_id = short_id();

        let span = Span {
            trace_id: parent.trace_id.clone(),
            span_id: span_id.clone(),
            parent_span_id: Some(parent.span_id.clone()),
            name: name.into(),
            status: SpanStatus::Active,
            start_time: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            end_time: None,
            duration_ms: None,
            attributes: HashMap::new(),
            events: Vec::new(),
        };

        self.spans.write().insert(span_id.clone(), span.clone());
        self.traces
            .write()
            .entry(parent.trace_id.clone())
            .or_default()
            .push(span_id);

        span
    }

    /// End a span
    pub fn end_span(&self, span_id: &str, status: SpanStatus) {
        if let Some(span) = self.spans.write().get_mut(span_id) {
            span.status = status;
            span.end_time =
                Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
            // Calculate duration from start/end
            if let (Ok(start), Some(ref end_str)) = (
                chrono::DateTime::parse_from_rfc3339(&span.start_time),
                &span.end_time,
            ) {
                if let Ok(end) = chrono::DateTime::parse_from_rfc3339(end_str) {
                    span.duration_ms = Some((end - start).num_milliseconds().max(0) as u64);
                }
            }
        }
    }

    /// Add an attribute to a span
    pub fn set_attribute(&self, span_id: &str, key: &str, value: serde_json::Value) {
        if let Some(span) = self.spans.write().get_mut(span_id) {
            span.attributes.insert(key.into(), value);
        }
    }

    /// Add an event to a span
    pub fn add_event(&self, span_id: &str, name: &str, attrs: HashMap<String, serde_json::Value>) {
        if let Some(span) = self.spans.write().get_mut(span_id) {
            span.events.push(SpanEvent {
                name: name.into(),
                timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                attributes: attrs,
            });
        }
    }

    /// Get a span by ID
    pub fn get_span(&self, span_id: &str) -> Option<Span> {
        self.spans.read().get(span_id).cloned()
    }

    /// Get all spans for a trace
    pub fn get_trace(&self, trace_id: &str) -> Vec<Span> {
        let traces = self.traces.read();
        let spans = self.spans.read();
        traces
            .get(trace_id)
            .map(|ids| ids.iter().filter_map(|id| spans.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Count active spans
    pub fn active_span_count(&self) -> usize {
        self.spans
            .read()
            .values()
            .filter(|s| s.status == SpanStatus::Active)
            .count()
    }

    /// Total span count
    pub fn span_count(&self) -> usize {
        self.spans.read().len()
    }

    /// Total trace count
    pub fn trace_count(&self) -> usize {
        self.traces.read().len()
    }
}

impl Default for TraceManager {
    fn default() -> Self {
        Self::new()
    }
}

fn short_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let tm = TraceManager::new();
        let span = tm.start_span("cognitive_loop");
        assert_eq!(span.status, SpanStatus::Active);
        assert!(span.parent_span_id.is_none());
        assert_eq!(tm.span_count(), 1);
        assert_eq!(tm.trace_count(), 1);
    }

    #[test]
    fn test_span_nesting() {
        let tm = TraceManager::new();
        let root = tm.start_span("request");
        let child1 = tm.start_child_span(&root, "perceive");
        let child2 = tm.start_child_span(&root, "think");
        let grandchild = tm.start_child_span(&child1, "llm_call");

        assert_eq!(child1.trace_id, root.trace_id);
        assert_eq!(child2.trace_id, root.trace_id);
        assert_eq!(grandchild.trace_id, root.trace_id);
        assert_eq!(child1.parent_span_id, Some(root.span_id.clone()));
        assert_eq!(grandchild.parent_span_id, Some(child1.span_id.clone()));

        assert_eq!(tm.span_count(), 4);
        assert_eq!(tm.trace_count(), 1);

        let trace_spans = tm.get_trace(&root.trace_id);
        assert_eq!(trace_spans.len(), 4);
    }

    #[test]
    fn test_span_end() {
        let tm = TraceManager::new();
        let span = tm.start_span("test");
        tm.end_span(&span.span_id, SpanStatus::Completed);

        let ended = tm.get_span(&span.span_id).unwrap();
        assert_eq!(ended.status, SpanStatus::Completed);
        assert!(ended.end_time.is_some());
        assert!(ended.duration_ms.is_some());
        assert_eq!(tm.active_span_count(), 0);
    }
}
