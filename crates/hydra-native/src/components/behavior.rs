//! Behavior settings tab component data (Step 3.9).
//!
//! Controls intent caching, belief revision, context compression,
//! sister routing, and proactive behavior settings.

use serde::{Deserialize, Serialize};

/// Compression aggressiveness for context windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    Aggressive,
    Balanced,
    Minimal,
}

/// Strategy for routing work across sisters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    Parallel,
    Sequential,
}

/// Frequency of proactive insight generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightFrequency {
    Rarely,
    Sometimes,
    Often,
}

/// Logical groupings within behavior settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorSection {
    IntentCache,
    BeliefRevision,
    ContextCompression,
    SisterRouting,
    ProactiveBehavior,
}

/// Full behavior configuration surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    // — Intent Cache —
    pub intent_cache_enabled: bool,
    pub intent_cache_ttl_hours: u32,
    pub intent_cache_size_limit: usize,

    // — Belief Revision —
    pub belief_revision_enabled: bool,
    pub belief_persistence_days: u32,
    pub editable_beliefs: bool,

    // — Context Compression —
    pub compression_level: CompressionLevel,
    pub token_budget_per_op: u32,

    // — Sister Routing —
    pub routing_strategy: RoutingStrategy,
    pub sister_timeout_secs: u32,
    pub retry_on_failure: bool,

    // — Proactive Behavior —
    pub proactive_enabled: bool,
    pub dream_state_enabled: bool,
    pub insight_frequency: InsightFrequency,
    pub notification_on_insight: bool,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            // Intent Cache
            intent_cache_enabled: true,
            intent_cache_ttl_hours: 1,
            intent_cache_size_limit: 1000,

            // Belief Revision
            belief_revision_enabled: true,
            belief_persistence_days: 30,
            editable_beliefs: false,

            // Context Compression
            compression_level: CompressionLevel::Balanced,
            token_budget_per_op: 4096,

            // Sister Routing
            routing_strategy: RoutingStrategy::Parallel,
            sister_timeout_secs: 10,
            retry_on_failure: true,

            // Proactive Behavior
            proactive_enabled: false,
            dream_state_enabled: false,
            insight_frequency: InsightFrequency::Sometimes,
            notification_on_insight: true,
        }
    }
}

impl BehaviorSettings {
    /// Number of logical sections in the behavior settings panel.
    pub fn section_count() -> usize {
        5
    }

    /// All sections in display order.
    pub fn sections() -> Vec<BehaviorSection> {
        vec![
            BehaviorSection::IntentCache,
            BehaviorSection::BeliefRevision,
            BehaviorSection::ContextCompression,
            BehaviorSection::SisterRouting,
            BehaviorSection::ProactiveBehavior,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let s = BehaviorSettings::default();
        assert!(s.intent_cache_enabled);
        assert_eq!(s.intent_cache_ttl_hours, 1);
        assert_eq!(s.intent_cache_size_limit, 1000);
        assert!(s.belief_revision_enabled);
        assert_eq!(s.belief_persistence_days, 30);
        assert!(!s.editable_beliefs);
        assert_eq!(s.compression_level, CompressionLevel::Balanced);
        assert_eq!(s.token_budget_per_op, 4096);
        assert_eq!(s.routing_strategy, RoutingStrategy::Parallel);
        assert_eq!(s.sister_timeout_secs, 10);
        assert!(s.retry_on_failure);
        assert!(!s.proactive_enabled);
        assert!(!s.dream_state_enabled);
        assert_eq!(s.insight_frequency, InsightFrequency::Sometimes);
        assert!(s.notification_on_insight);
    }

    #[test]
    fn test_section_count() {
        assert_eq!(BehaviorSettings::section_count(), 5);
    }

    #[test]
    fn test_sections_list() {
        let sections = BehaviorSettings::sections();
        assert_eq!(sections.len(), 5);
        assert_eq!(sections[0], BehaviorSection::IntentCache);
        assert_eq!(sections[1], BehaviorSection::BeliefRevision);
        assert_eq!(sections[2], BehaviorSection::ContextCompression);
        assert_eq!(sections[3], BehaviorSection::SisterRouting);
        assert_eq!(sections[4], BehaviorSection::ProactiveBehavior);
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = BehaviorSettings::default();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: BehaviorSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.intent_cache_ttl_hours, original.intent_cache_ttl_hours);
        assert_eq!(restored.compression_level, original.compression_level);
        assert_eq!(restored.routing_strategy, original.routing_strategy);
        assert_eq!(restored.insight_frequency, original.insight_frequency);
    }

    #[test]
    fn test_serde_enums() {
        let json = serde_json::to_string(&CompressionLevel::Aggressive).unwrap();
        assert_eq!(json, "\"Aggressive\"");

        let json = serde_json::to_string(&RoutingStrategy::Sequential).unwrap();
        assert_eq!(json, "\"Sequential\"");

        let json = serde_json::to_string(&InsightFrequency::Often).unwrap();
        assert_eq!(json, "\"Often\"");
    }

    #[test]
    fn test_custom_config() {
        let mut s = BehaviorSettings::default();
        s.intent_cache_enabled = false;
        s.compression_level = CompressionLevel::Aggressive;
        s.routing_strategy = RoutingStrategy::Sequential;
        s.proactive_enabled = true;
        s.dream_state_enabled = true;
        s.insight_frequency = InsightFrequency::Often;

        let json = serde_json::to_string(&s).unwrap();
        let restored: BehaviorSettings = serde_json::from_str(&json).unwrap();
        assert!(!restored.intent_cache_enabled);
        assert_eq!(restored.compression_level, CompressionLevel::Aggressive);
        assert_eq!(restored.routing_strategy, RoutingStrategy::Sequential);
        assert!(restored.proactive_enabled);
        assert!(restored.dream_state_enabled);
        assert_eq!(restored.insight_frequency, InsightFrequency::Often);
    }
}
