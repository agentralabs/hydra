//! hydra-observability — Structured logging, metrics, and distributed tracing.
//!
//! - **Logger**: JSON structured logs with context
//! - **Metrics**: Counters, gauges, histograms (Prometheus format)
//! - **Tracing**: Spans with trace/span IDs
//! - **Exporter**: File, stdout, remote output
//! - **Filter**: Level-based filtering and sampling
//! - **Context**: Correlation IDs and log context

pub mod context;
pub mod exporter;
pub mod filter;
pub mod logger;
pub mod metrics;
pub mod traces;
