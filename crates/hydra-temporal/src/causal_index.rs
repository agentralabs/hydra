//! Causal chain index — maps causal roots to memory IDs.

use crate::btree::MemoryId;
use crate::constants::CAUSAL_INDEX_MAX_ROOTS;
use std::collections::HashMap;

/// An index that maps causal root identifiers to sets of memory IDs.
///
/// Every memory can be traced back to a causal root, forming a causal
/// chain. This index makes it efficient to find all memories that
/// descend from the same root cause.
pub struct CausalChainIndex {
    roots: HashMap<String, Vec<MemoryId>>,
}

impl Default for CausalChainIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl CausalChainIndex {
    /// Create a new, empty causal chain index.
    pub fn new() -> Self {
        Self {
            roots: HashMap::new(),
        }
    }

    /// Insert a memory under a causal root.
    ///
    /// Returns `false` if the maximum number of roots has been reached
    /// and a new root would need to be created.
    pub fn insert(&mut self, root: String, memory_id: MemoryId) -> bool {
        if !self.roots.contains_key(&root) && self.roots.len() >= CAUSAL_INDEX_MAX_ROOTS {
            return false;
        }
        self.roots.entry(root).or_default().push(memory_id);
        true
    }

    /// Get all memory IDs associated with a causal root.
    pub fn memories_for_root(&self, root: &str) -> Vec<&MemoryId> {
        self.roots
            .get(root)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Return the number of distinct causal roots.
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Return the total number of indexed memory-root associations.
    pub fn total_indexed(&self) -> usize {
        self.roots.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_lookup() {
        let mut idx = CausalChainIndex::new();
        idx.insert("root-1".to_string(), MemoryId::from_value("mem-1"));
        idx.insert("root-1".to_string(), MemoryId::from_value("mem-2"));
        let mems = idx.memories_for_root("root-1");
        assert_eq!(mems.len(), 2);
    }

    #[test]
    fn root_count_tracking() {
        let mut idx = CausalChainIndex::new();
        idx.insert("a".to_string(), MemoryId::from_value("m1"));
        idx.insert("b".to_string(), MemoryId::from_value("m2"));
        assert_eq!(idx.root_count(), 2);
        assert_eq!(idx.total_indexed(), 2);
    }
}
