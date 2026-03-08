//! LogContext — correlation IDs and structured context for log entries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Context attached to log entries for correlation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogContext {
    /// Unique trace ID for request correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Span ID within the trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// Parent span ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    /// Run ID for cognitive loop correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Component that generated the log
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// Cognitive loop phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// Additional key-value attributes
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, serde_json::Value>,
}

impl LogContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_trace(mut self, trace_id: &str) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_span(mut self, span_id: &str) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    pub fn with_parent_span(mut self, parent_id: &str) -> Self {
        self.parent_span_id = Some(parent_id.into());
        self
    }

    pub fn with_run(mut self, run_id: &str) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    pub fn with_component(mut self, component: &str) -> Self {
        self.component = Some(component.into());
        self
    }

    pub fn with_phase(mut self, phase: &str) -> Self {
        self.phase = Some(phase.into());
        self
    }

    pub fn with_attr(mut self, key: &str, value: serde_json::Value) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    /// Generate a new trace ID
    pub fn new_trace() -> Self {
        Self {
            trace_id: Some(uuid::Uuid::new_v4().to_string()),
            span_id: Some(generate_span_id()),
            ..Default::default()
        }
    }

    /// Create a child context (inherits trace_id, new span_id)
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Some(generate_span_id()),
            parent_span_id: self.span_id.clone(),
            run_id: self.run_id.clone(),
            component: self.component.clone(),
            phase: None,
            attributes: HashMap::new(),
        }
    }
}

/// Generate a short span ID
fn generate_span_id() -> String {
    let id = uuid::Uuid::new_v4();
    id.to_string()[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = LogContext::new()
            .with_trace("trace-123")
            .with_span("span-456")
            .with_run("run-789")
            .with_component("cognitive_loop")
            .with_phase("think")
            .with_attr("tokens", serde_json::json!(150));

        assert_eq!(ctx.trace_id.as_deref(), Some("trace-123"));
        assert_eq!(ctx.span_id.as_deref(), Some("span-456"));
        assert_eq!(ctx.run_id.as_deref(), Some("run-789"));
        assert_eq!(ctx.component.as_deref(), Some("cognitive_loop"));
        assert_eq!(ctx.phase.as_deref(), Some("think"));
        assert_eq!(ctx.attributes.get("tokens"), Some(&serde_json::json!(150)));
    }

    #[test]
    fn test_child_context() {
        let parent = LogContext::new_trace()
            .with_component("server")
            .with_run("run-1");

        let child = parent.child().with_phase("perceive");

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.parent_span_id, parent.span_id);
        assert_eq!(child.run_id, parent.run_id);
        assert_eq!(child.phase.as_deref(), Some("perceive"));
    }

    #[test]
    fn test_correlation_id_generation() {
        let ctx = LogContext::new_trace();
        assert!(ctx.trace_id.is_some());
        assert!(ctx.span_id.is_some());

        let ctx2 = LogContext::new_trace();
        assert_ne!(ctx.trace_id, ctx2.trace_id);
    }
}
