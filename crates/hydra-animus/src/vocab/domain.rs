//! Domain vocabulary registration.

use crate::{
    constants::{DOMAIN_MAX_COUNT, DOMAIN_VOCAB_MAX_ENTRIES},
    errors::AnimusError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A domain's contributed vocabulary entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabEntry {
    /// The name of this type within the domain.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Example JSON payload for nodes of this type.
    pub example: serde_json::Value,
}

/// All vocabulary registered by a single domain skill.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DomainVocab {
    /// Domain name.
    pub domain: String,
    /// Node types contributed by this domain.
    pub node_types: Vec<VocabEntry>,
    /// Edge types contributed by this domain.
    pub edge_types: Vec<VocabEntry>,
}

impl DomainVocab {
    /// Create a new domain vocabulary.
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            node_types: Vec::new(),
            edge_types: Vec::new(),
        }
    }

    /// Add a node type to this vocabulary.
    pub fn with_node_type(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        example: serde_json::Value,
    ) -> Self {
        self.node_types.push(VocabEntry {
            name: name.into(),
            description: description.into(),
            example,
        });
        self
    }

    /// Add an edge type to this vocabulary.
    pub fn with_edge_type(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        example: serde_json::Value,
    ) -> Self {
        self.edge_types.push(VocabEntry {
            name: name.into(),
            description: description.into(),
            example,
        });
        self
    }
}

/// The vocabulary registry — tracks all registered domain vocabularies.
#[derive(Debug, Default)]
pub struct VocabRegistry {
    domains: HashMap<String, DomainVocab>,
}

impl VocabRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a domain vocabulary.
    pub fn register(&mut self, vocab: DomainVocab) -> Result<(), AnimusError> {
        if self.domains.len() >= DOMAIN_MAX_COUNT && !self.domains.contains_key(&vocab.domain) {
            return Err(AnimusError::VocabRegistrationFailed {
                domain: vocab.domain.clone(),
                reason: format!("maximum domain count {} reached", DOMAIN_MAX_COUNT),
            });
        }

        let total_entries = vocab.node_types.len() + vocab.edge_types.len();

        if total_entries > DOMAIN_VOCAB_MAX_ENTRIES {
            return Err(AnimusError::VocabRegistrationFailed {
                domain: vocab.domain.clone(),
                reason: format!(
                    "vocabulary has {} entries, maximum is {}",
                    total_entries, DOMAIN_VOCAB_MAX_ENTRIES
                ),
            });
        }

        self.domains.insert(vocab.domain.clone(), vocab);
        Ok(())
    }

    /// Returns true if a domain is registered.
    pub fn has_domain(&self, domain: &str) -> bool {
        self.domains.contains_key(domain)
    }

    /// Returns the vocabulary for a domain.
    pub fn get(&self, domain: &str) -> Option<&DomainVocab> {
        self.domains.get(domain)
    }

    /// Total number of registered domains.
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Returns true if a node type name is known in a domain.
    pub fn is_known_node_type(&self, domain: &str, name: &str) -> bool {
        self.domains
            .get(domain)
            .map(|v| v.node_types.iter().any(|e| e.name == name))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_retrieve_domain() {
        let mut reg = VocabRegistry::new();
        let vocab = DomainVocab::new("finance")
            .with_node_type("Trade", "A financial trade", serde_json::json!({}))
            .with_edge_type("Executes", "Order executes trade", serde_json::json!({}));

        reg.register(vocab).unwrap();
        assert!(reg.has_domain("finance"));
        assert_eq!(reg.domain_count(), 1);
    }

    #[test]
    fn duplicate_domain_not_rejected() {
        let mut reg = VocabRegistry::new();
        let v1 = DomainVocab::new("finance");
        let v2 = DomainVocab::new("finance");
        reg.register(v1).unwrap();
        reg.register(v2).unwrap(); // should succeed
        assert_eq!(reg.domain_count(), 1);
    }

    #[test]
    fn known_node_type_check() {
        let mut reg = VocabRegistry::new();
        let vocab =
            DomainVocab::new("finance").with_node_type("Trade", "desc", serde_json::json!({}));
        reg.register(vocab).unwrap();
        assert!(reg.is_known_node_type("finance", "Trade"));
        assert!(!reg.is_known_node_type("finance", "Unknown"));
    }
}
