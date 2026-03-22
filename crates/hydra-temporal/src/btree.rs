//! Append-only chrono-spatial B+ tree backed by `BTreeMap`.
//!
//! No delete. No overwrite. Ever.

use crate::constants::{NANOS_PER_SECOND, RANGE_QUERY_MAX_DAYS, RECENT_CACHE_SIZE};
use crate::errors::TemporalError;
use crate::timestamp::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Unique identifier for a memory entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(String);

impl MemoryId {
    /// Create a new `MemoryId` from a string value.
    pub fn from_value(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A coordinate in the manifold space (3D for now).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ManifoldCoord {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
}

impl ManifoldCoord {
    /// Create a new manifold coordinate.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean distance to another coordinate.
    pub fn distance(&self, other: &ManifoldCoord) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A single entry in the chrono-spatial B+ tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEntry {
    /// The nanosecond timestamp (key).
    pub timestamp: Timestamp,
    /// The memory this entry refers to.
    pub memory_id: MemoryId,
    /// Position in the manifold.
    pub coord: ManifoldCoord,
    /// Optional causal root identifier.
    pub causal_root: Option<String>,
}

/// Append-only chrono-spatial B+ tree.
///
/// Uses a `BTreeMap<u64, TemporalEntry>` internally for ordered access.
/// A recent-entry cache provides O(1) access to the latest entries.
pub struct ChronoSpatialBTree {
    tree: BTreeMap<u64, TemporalEntry>,
    recent: VecDeque<TemporalEntry>,
}

impl Default for ChronoSpatialBTree {
    fn default() -> Self {
        Self::new()
    }
}

impl ChronoSpatialBTree {
    /// Create a new, empty chrono-spatial B+ tree.
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
            recent: VecDeque::with_capacity(RECENT_CACHE_SIZE),
        }
    }

    /// Insert a new entry. Duplicates are rejected (append-only).
    pub fn insert(&mut self, entry: TemporalEntry) -> Result<(), TemporalError> {
        let key = entry.timestamp.as_nanos();
        if self.tree.contains_key(&key) {
            return Err(TemporalError::DuplicateTimestamp(key));
        }
        self.tree.insert(key, entry.clone());
        self.recent.push_front(entry);
        if self.recent.len() > RECENT_CACHE_SIZE {
            self.recent.pop_back();
        }
        Ok(())
    }

    /// Get an entry by exact timestamp.
    pub fn get_exact(&self, ts: &Timestamp) -> Option<&TemporalEntry> {
        self.tree.get(&ts.as_nanos())
    }

    /// Get the entry nearest to the given timestamp.
    pub fn get_nearest(&self, ts: &Timestamp) -> Option<&TemporalEntry> {
        let key = ts.as_nanos();
        let before = self.tree.range(..=key).next_back();
        let after = self.tree.range(key..).next();

        match (before, after) {
            (Some((kb, vb)), Some((ka, va))) => {
                if key.abs_diff(*kb) <= key.abs_diff(*ka) {
                    Some(vb)
                } else {
                    Some(va)
                }
            }
            (Some((_, v)), None) | (None, Some((_, v))) => Some(v),
            (None, None) => None,
        }
    }

    /// Scan entries in the given timestamp range (inclusive).
    ///
    /// Returns an error if the range exceeds `RANGE_QUERY_MAX_DAYS`.
    pub fn range_scan(
        &self,
        from: &Timestamp,
        to: &Timestamp,
    ) -> Result<Vec<&TemporalEntry>, TemporalError> {
        let span_ns = from.delta_nanos(to);
        let span_days = span_ns / (NANOS_PER_SECOND * 86_400);
        if span_days > RANGE_QUERY_MAX_DAYS {
            return Err(TemporalError::RangeQueryTooLarge {
                days: span_days,
                max_days: RANGE_QUERY_MAX_DAYS,
            });
        }
        let lo = from.as_nanos().min(to.as_nanos());
        let hi = from.as_nanos().max(to.as_nanos());
        Ok(self.tree.range(lo..=hi).map(|(_, v)| v).collect())
    }

    /// Return the most recent entries from the cache (O(1) per entry).
    pub fn most_recent(&self, n: usize) -> Vec<&TemporalEntry> {
        self.recent.iter().take(n).collect()
    }

    /// Return entries whose manifold coordinate is within `radius` of `center`.
    pub fn spatial_range(&self, center: &ManifoldCoord, radius: f64) -> Vec<&TemporalEntry> {
        self.tree
            .values()
            .filter(|e| e.coord.distance(center) <= radius)
            .collect()
    }

    /// Total number of entries in the tree.
    pub fn len(&self) -> usize {
        self.tree.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(nanos: u64) -> TemporalEntry {
        TemporalEntry {
            timestamp: Timestamp::from_nanos(nanos).unwrap(),
            memory_id: MemoryId::from_value(format!("mem-{nanos}")),
            coord: ManifoldCoord::new(0.0, 0.0, 0.0),
            causal_root: None,
        }
    }

    #[test]
    fn insert_and_retrieve() {
        let mut tree = ChronoSpatialBTree::new();
        tree.insert(make_entry(100)).unwrap();
        assert!(tree
            .get_exact(&Timestamp::from_nanos(100).unwrap())
            .is_some());
    }

    #[test]
    fn duplicate_rejected() {
        let mut tree = ChronoSpatialBTree::new();
        tree.insert(make_entry(100)).unwrap();
        assert!(tree.insert(make_entry(100)).is_err());
    }

    #[test]
    fn nearest_lookup() {
        let mut tree = ChronoSpatialBTree::new();
        tree.insert(make_entry(100)).unwrap();
        tree.insert(make_entry(200)).unwrap();
        tree.insert(make_entry(300)).unwrap();
        let nearest = tree
            .get_nearest(&Timestamp::from_nanos(190).unwrap())
            .unwrap();
        assert_eq!(nearest.timestamp.as_nanos(), 200);
    }

    #[test]
    fn range_scan_basic() {
        let mut tree = ChronoSpatialBTree::new();
        for i in 1..=10 {
            tree.insert(make_entry(i * 1000)).unwrap();
        }
        let from = Timestamp::from_nanos(3000).unwrap();
        let to = Timestamp::from_nanos(7000).unwrap();
        let results = tree.range_scan(&from, &to).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn most_recent_returns_latest() {
        let mut tree = ChronoSpatialBTree::new();
        for i in 1..=5 {
            tree.insert(make_entry(i * 1000)).unwrap();
        }
        let recent = tree.most_recent(2);
        assert_eq!(recent.len(), 2);
        // Most recent should be 5000, then 4000
        assert_eq!(recent[0].timestamp.as_nanos(), 5000);
        assert_eq!(recent[1].timestamp.as_nanos(), 4000);
    }
}
