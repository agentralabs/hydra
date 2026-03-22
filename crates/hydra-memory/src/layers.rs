//! The 8 memory layers and their mapping to CognitiveEvent types.
//! Every memory in Hydra is one of these 8 types.
//! All are stored as CognitiveEvents in AgenticMemory.

use crate::constants::*;
use agentic_memory::EventType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Which of the 8 memory layers a record belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryLayer {
    /// Exact verbatim record of an exchange. Immutable.
    Verbatim,
    /// Episodic context — the experience of an interaction.
    Episodic,
    /// Extracted semantic facts. Subject to AGM revision.
    Semantic,
    /// Entity relationship graph updates.
    Relational,
    /// Causal structure — what caused what.
    Causal,
    /// Procedural knowledge — how to do things.
    Procedural,
    /// Anticipatory patterns — what comes next.
    Anticipatory,
    /// Identity memory — who the principal is.
    Identity,
}

impl MemoryLayer {
    /// The tag used in AgenticMemory content to identify this layer.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Verbatim => LAYER_VERBATIM,
            Self::Episodic => LAYER_EPISODIC,
            Self::Semantic => LAYER_SEMANTIC,
            Self::Relational => LAYER_RELATIONAL,
            Self::Causal => LAYER_CAUSAL,
            Self::Procedural => LAYER_PROCEDURAL,
            Self::Anticipatory => LAYER_ANTICIPATORY,
            Self::Identity => LAYER_IDENTITY,
        }
    }

    /// The AgenticMemory EventType for this layer.
    pub fn event_type(&self) -> EventType {
        match self {
            // Verbatim = Fact (immutable, ground truth)
            Self::Verbatim => EventType::Fact,
            // Episodic = Episode (a coherent experience)
            Self::Episodic => EventType::Episode,
            // Semantic = Inference (extracted knowledge)
            Self::Semantic => EventType::Inference,
            // Relational = Fact (relationship graph update)
            Self::Relational => EventType::Fact,
            // Causal = Decision (what caused what)
            Self::Causal => EventType::Decision,
            // Procedural = Skill (how to do things)
            Self::Procedural => EventType::Skill,
            // Anticipatory = Inference (pattern-derived prediction)
            Self::Anticipatory => EventType::Inference,
            // Identity = Correction (updating self-model)
            Self::Identity => EventType::Correction,
        }
    }

    /// True if this layer's records are subject to AGM revision.
    pub fn is_revisable(&self) -> bool {
        matches!(self, Self::Semantic | Self::Anticipatory | Self::Identity)
    }

    /// True if this layer's records are immutable (Constitutional Law 1).
    pub fn is_immutable(&self) -> bool {
        matches!(self, Self::Verbatim)
    }
}

/// A memory record ready to be stored in AgenticMemory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Unique ID for this record.
    pub id: String,
    /// Which layer this belongs to.
    pub layer: MemoryLayer,
    /// The content (serialized as JSON for storage).
    pub content: serde_json::Value,
    /// When this memory was created.
    pub created_at: DateTime<Utc>,
    /// Session this memory belongs to.
    pub session_id: String,
    /// Causal chain root (links to hydra-temporal causal index).
    pub causal_root: String,
    /// Optional SHA256 hash (required for Verbatim layer).
    pub content_hash: Option<String>,
}

impl MemoryRecord {
    /// Create a new memory record.
    pub fn new(
        layer: MemoryLayer,
        content: serde_json::Value,
        session_id: impl Into<String>,
        causal_root: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            layer,
            content,
            created_at: Utc::now(),
            session_id: session_id.into(),
            causal_root: causal_root.into(),
            content_hash: None,
        }
    }

    /// Set the content hash (required for Verbatim layer).
    pub fn with_hash(mut self, hash: String) -> Self {
        self.content_hash = Some(hash);
        self
    }

    /// Build a CognitiveEvent content string for AgenticMemory storage.
    /// The content is prefixed with the layer tag for filtering.
    pub fn to_cognitive_content(&self) -> String {
        format!(
            "{} | session:{} | causal:{} | {}",
            self.layer.tag(),
            self.session_id,
            self.causal_root,
            self.content
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_is_immutable() {
        assert!(MemoryLayer::Verbatim.is_immutable());
        assert!(!MemoryLayer::Semantic.is_immutable());
    }

    #[test]
    fn semantic_is_revisable() {
        assert!(MemoryLayer::Semantic.is_revisable());
        assert!(!MemoryLayer::Verbatim.is_revisable());
    }

    #[test]
    fn all_layers_have_tags() {
        let layers = [
            MemoryLayer::Verbatim,
            MemoryLayer::Episodic,
            MemoryLayer::Semantic,
            MemoryLayer::Relational,
            MemoryLayer::Causal,
            MemoryLayer::Procedural,
            MemoryLayer::Anticipatory,
            MemoryLayer::Identity,
        ];
        for layer in &layers {
            assert!(!layer.tag().is_empty());
        }
    }

    #[test]
    fn cognitive_content_contains_layer_tag() {
        let record = MemoryRecord::new(
            MemoryLayer::Verbatim,
            serde_json::json!({"input": "hello"}),
            "session-001",
            "const-identity",
        );
        let content = record.to_cognitive_content();
        assert!(content.contains(LAYER_VERBATIM));
        assert!(content.contains("session-001"));
    }
}
