//! Belief store — the in-memory set of all beliefs.

use crate::belief::{Belief, BeliefCategory};
use crate::constants::BELIEF_SET_MAX_SIZE;
use crate::errors::BeliefError;
use std::collections::HashMap;

/// The in-memory store of all beliefs.
pub struct BeliefStore {
    beliefs: HashMap<String, Belief>,
}

impl BeliefStore {
    /// Create a new empty belief store.
    pub fn new() -> Self {
        Self {
            beliefs: HashMap::new(),
        }
    }

    /// Insert a belief into the store.
    pub fn insert(&mut self, belief: Belief) -> Result<(), BeliefError> {
        if self.beliefs.len() >= BELIEF_SET_MAX_SIZE {
            return Err(BeliefError::BeliefSetFull {
                max: BELIEF_SET_MAX_SIZE,
            });
        }
        self.beliefs.insert(belief.id.clone(), belief);
        Ok(())
    }

    /// Get a belief by ID.
    pub fn get(&self, id: &str) -> Option<&Belief> {
        self.beliefs.get(id)
    }

    /// Get a mutable reference to a belief by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Belief> {
        self.beliefs.get_mut(id)
    }

    /// Return all beliefs as a vector.
    pub fn all(&self) -> Vec<&Belief> {
        self.beliefs.values().collect()
    }

    /// Find beliefs whose proposition contains the given keyword.
    pub fn find_by_keyword(&self, keyword: &str) -> Vec<&Belief> {
        let lower = keyword.to_lowercase();
        self.beliefs
            .values()
            .filter(|b| b.proposition.to_lowercase().contains(&lower))
            .collect()
    }

    /// Return all capability beliefs.
    pub fn capability_beliefs(&self) -> Vec<&Belief> {
        self.beliefs
            .values()
            .filter(|b| b.category == BeliefCategory::Capability)
            .collect()
    }

    /// Return the number of beliefs in the store.
    pub fn len(&self) -> usize {
        self.beliefs.len()
    }

    /// Return true if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.beliefs.is_empty()
    }
}

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::Belief;

    #[test]
    fn insert_and_get() {
        let mut store = BeliefStore::new();
        let b = Belief::world("test proposition", 0.7);
        let id = b.id.clone();
        store.insert(b).unwrap();
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn find_by_keyword() {
        let mut store = BeliefStore::new();
        store
            .insert(Belief::world("the weather is nice", 0.8))
            .unwrap();
        store
            .insert(Belief::world("code quality is high", 0.9))
            .unwrap();
        let results = store.find_by_keyword("weather");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn capability_beliefs_filter() {
        let mut store = BeliefStore::new();
        store.insert(Belief::world("world fact", 0.5)).unwrap();
        store.insert(Belief::capability("can reason", 0.8)).unwrap();
        assert_eq!(store.capability_beliefs().len(), 1);
    }
}
