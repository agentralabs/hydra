//! Three-tier response system: Instant (<100ms), Fast (<500ms), Full (<3s).

use serde::{Deserialize, Serialize};

/// The three response tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseTier {
    /// <100ms — cached, predicted, or acknowledged
    Instant,
    /// <500ms — local LLM rough answer
    Fast,
    /// <3s — cloud LLM complete response
    Full,
}

impl ResponseTier {
    /// Target latency in milliseconds
    pub fn target_ms(&self) -> u64 {
        match self {
            Self::Instant => 100,
            Self::Fast => 500,
            Self::Full => 3000,
        }
    }

    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Instant => "instant",
            Self::Fast => "fast",
            Self::Full => "full",
        }
    }

    /// Escalate to the next tier
    pub fn escalate(&self) -> Option<ResponseTier> {
        match self {
            Self::Instant => Some(Self::Fast),
            Self::Fast => Some(Self::Full),
            Self::Full => None,
        }
    }
}

/// A response with its tier and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredResponse {
    pub tier: ResponseTier,
    pub content: String,
    pub confidence: f64,
    pub latency_ms: u64,
    pub source: ResponseSource,
    /// Whether a higher-tier response is incoming
    pub superseded_by: Option<ResponseTier>,
}

/// Where the response came from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseSource {
    /// From prediction cache
    Cache,
    /// From response predictor
    Predicted,
    /// Acknowledgment (no real content yet)
    Ack,
    /// From local LLM (Ollama)
    LocalLlm,
    /// From cloud LLM (Claude)
    CloudLlm,
}

/// Configuration for tier selection
#[derive(Debug, Clone)]
pub struct TierConfig {
    /// Whether instant tier is enabled
    pub instant_enabled: bool,
    /// Whether fast tier is enabled (requires local LLM)
    pub fast_enabled: bool,
    /// Minimum confidence to use a cached/predicted response
    pub min_cache_confidence: f64,
    /// Whether to always escalate (send all tiers progressively)
    pub progressive: bool,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            instant_enabled: true,
            fast_enabled: true,
            min_cache_confidence: 0.7,
            progressive: true,
        }
    }
}

/// Selects the appropriate tier for a given request
pub struct TierSelector {
    config: TierConfig,
}

impl TierSelector {
    pub fn new(config: TierConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(TierConfig::default())
    }

    /// Determine which tier to start with for a given input
    pub fn select(
        &self,
        cache_hit: bool,
        cache_confidence: f64,
        local_llm_available: bool,
    ) -> ResponseTier {
        // Try instant first
        if self.config.instant_enabled
            && cache_hit
            && cache_confidence >= self.config.min_cache_confidence
        {
            return ResponseTier::Instant;
        }

        // Try fast (local LLM)
        if self.config.fast_enabled && local_llm_available {
            return ResponseTier::Fast;
        }

        // Fall through to full
        ResponseTier::Full
    }

    /// Whether progressive escalation is enabled
    pub fn is_progressive(&self) -> bool {
        self.config.progressive
    }

    /// Get the escalation chain for a starting tier
    pub fn escalation_chain(&self, start: ResponseTier) -> Vec<ResponseTier> {
        let mut chain = vec![start];
        let mut current = start;
        while let Some(next) = current.escalate() {
            chain.push(next);
            current = next;
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_ordering() {
        assert!(ResponseTier::Instant < ResponseTier::Fast);
        assert!(ResponseTier::Fast < ResponseTier::Full);
    }

    #[test]
    fn test_tier_target_latency() {
        assert_eq!(ResponseTier::Instant.target_ms(), 100);
        assert_eq!(ResponseTier::Fast.target_ms(), 500);
        assert_eq!(ResponseTier::Full.target_ms(), 3000);
    }

    #[test]
    fn test_tier_escalation() {
        assert_eq!(ResponseTier::Instant.escalate(), Some(ResponseTier::Fast));
        assert_eq!(ResponseTier::Fast.escalate(), Some(ResponseTier::Full));
        assert_eq!(ResponseTier::Full.escalate(), None);
    }

    #[test]
    fn test_tier_selection_cache_hit() {
        let selector = TierSelector::with_defaults();
        let tier = selector.select(true, 0.9, true);
        assert_eq!(tier, ResponseTier::Instant);
    }

    #[test]
    fn test_tier_selection_low_confidence() {
        let selector = TierSelector::with_defaults();
        // Cache hit but low confidence — skip to fast
        let tier = selector.select(true, 0.3, true);
        assert_eq!(tier, ResponseTier::Fast);
    }

    #[test]
    fn test_tier_selection_no_cache_no_local() {
        let selector = TierSelector::with_defaults();
        let tier = selector.select(false, 0.0, false);
        assert_eq!(tier, ResponseTier::Full);
    }

    #[test]
    fn test_tier_selection_no_cache_with_local() {
        let selector = TierSelector::with_defaults();
        let tier = selector.select(false, 0.0, true);
        assert_eq!(tier, ResponseTier::Fast);
    }

    #[test]
    fn test_escalation_chain() {
        let selector = TierSelector::with_defaults();
        let chain = selector.escalation_chain(ResponseTier::Instant);
        assert_eq!(
            chain,
            vec![
                ResponseTier::Instant,
                ResponseTier::Fast,
                ResponseTier::Full
            ]
        );
    }

    #[test]
    fn test_escalation_chain_from_full() {
        let selector = TierSelector::with_defaults();
        let chain = selector.escalation_chain(ResponseTier::Full);
        assert_eq!(chain, vec![ResponseTier::Full]);
    }
}
