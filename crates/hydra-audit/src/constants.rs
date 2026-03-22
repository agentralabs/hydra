//! Audit system constants — all tunable values live here.

/// Maximum audit records stored in memory.
pub const MAX_AUDIT_RECORDS: usize = 100_000;

/// Maximum events in one execution trace.
pub const MAX_TRACE_EVENTS: usize = 500;

/// Narrative max length (characters).
pub const MAX_NARRATIVE_CHARS: usize = 4_096;

/// Maximum records returned in a single query.
pub const MAX_QUERY_RESULTS: usize = 1_000;

/// Audit record hash label.
pub const AUDIT_HASH_LABEL: &str = "sha256-audit";
