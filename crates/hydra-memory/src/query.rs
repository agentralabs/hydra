//! Query dispatch — routes memory queries to AgenticMemory or hydra-temporal.

use crate::bridge::HydraMemoryBridge;
use hydra_temporal::timestamp::Timestamp;

/// The result of a memory query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Memory IDs that matched the query.
    pub memory_ids: Vec<String>,
    /// Total number of results found.
    pub total_found: usize,
    /// Which type of query was run.
    pub query_type: String,
}

impl QueryResult {
    fn empty(query_type: &str) -> Self {
        Self {
            memory_ids: vec![],
            total_found: 0,
            query_type: query_type.to_string(),
        }
    }

    fn from_ids(ids: Vec<String>, query_type: &str) -> Self {
        let total = ids.len();
        Self {
            memory_ids: ids,
            total_found: total,
            query_type: query_type.to_string(),
        }
    }
}

/// Dispatch a query for memories at an exact timestamp.
pub fn query_exact_timestamp(bridge: &HydraMemoryBridge, ts: &Timestamp) -> QueryResult {
    match bridge.at_timestamp(ts) {
        Some(id) => QueryResult::from_ids(vec![id], "exact_timestamp"),
        None => QueryResult::empty("exact_timestamp"),
    }
}

/// Dispatch a query for the most recent N memories.
pub fn query_most_recent(bridge: &HydraMemoryBridge, n: usize) -> QueryResult {
    QueryResult::from_ids(bridge.recent(n), "most_recent")
}

/// Dispatch a query for memories in a time range.
pub fn query_time_range(
    bridge: &HydraMemoryBridge,
    start: &Timestamp,
    end: &Timestamp,
) -> QueryResult {
    match bridge.temporal.range_scan(start, end) {
        Ok(entries) => {
            let ids: Vec<String> = entries.iter().map(|e| e.memory_id.to_string()).collect();
            QueryResult::from_ids(ids, "time_range")
        }
        Err(_) => QueryResult::empty("time_range"),
    }
}

/// Dispatch a query for memories by causal root.
pub fn query_causal_root(bridge: &HydraMemoryBridge, root: &str) -> QueryResult {
    let entries = bridge.temporal.by_causal_root(root);
    let ids: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
    QueryResult::from_ids(ids, "causal_root")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Query tests require a live bridge — tested in integration tests.
    #[test]
    fn query_result_empty() {
        let q = QueryResult::empty("test");
        assert_eq!(q.total_found, 0);
        assert!(q.memory_ids.is_empty());
    }

    #[test]
    fn query_result_from_ids() {
        let q = QueryResult::from_ids(vec!["id-1".to_string(), "id-2".to_string()], "test");
        assert_eq!(q.total_found, 2);
    }
}
