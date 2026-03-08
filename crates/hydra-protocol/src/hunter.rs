use std::time::Duration;

use hydra_core::error::HydraError;

use crate::health::HealthStatus;
use crate::registry::ProtocolRegistry;
use crate::types::ProtocolEntry;

/// Ranked protocol with score
#[derive(Debug, Clone)]
pub struct RankedProtocol {
    pub protocol: ProtocolEntry,
    pub score: f64,
    pub rank: usize,
}

/// Discovery result
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub primary: Option<RankedProtocol>,
    pub fallbacks: Vec<RankedProtocol>,
    pub manual_guidance: Option<String>,
}

impl DiscoveryResult {
    pub fn empty() -> Self {
        Self {
            primary: None,
            fallbacks: vec![],
            manual_guidance: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.primary.is_none()
    }

    pub fn is_fallback(&self) -> bool {
        self.primary.is_none() && !self.fallbacks.is_empty()
    }

    pub fn with_manual(mut self, guidance: impl Into<String>) -> Self {
        self.manual_guidance = Some(guidance.into());
        self
    }
}

/// Protocol hunter — discovers and ranks protocols for a given action
pub struct ProtocolHunter {
    registry: ProtocolRegistry,
    timeout: Duration,
}

impl ProtocolHunter {
    pub fn new(registry: ProtocolRegistry) -> Self {
        Self {
            registry,
            timeout: Duration::from_secs(5),
        }
    }

    pub fn registry(&self) -> &ProtocolRegistry {
        &self.registry
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Discover protocols that can handle a capability, ranked by efficiency
    pub fn discover(&self, capability: &str) -> Result<DiscoveryResult, HydraError> {
        let candidates = self.registry.find_by_capability(capability);

        if candidates.is_empty() {
            // Check if any protocol exists but is unhealthy
            let all = self.registry.list_all();
            let has_unhealthy = all.iter().any(|p| {
                p.can_handle(capability)
                    && matches!(
                        self.registry.check_health(p.id),
                        HealthStatus::Unhealthy | HealthStatus::Degraded
                    )
            });

            if has_unhealthy {
                return Err(HydraError::AllProtocolsFailed(
                    "All protocols for this capability are unhealthy".into(),
                ));
            }

            return Ok(DiscoveryResult::empty().with_manual(format!(
                "No protocol found for capability: {capability}. Consider manual steps."
            )));
        }

        // Check for circular dependencies
        for candidate in &candidates {
            if self.registry.has_circular_dependency(candidate.id) {
                return Err(HydraError::Internal(
                    "Circular protocol dependency detected".into(),
                ));
            }
        }

        // Rank by efficiency score (capability / token_cost)
        let mut ranked: Vec<RankedProtocol> = candidates
            .into_iter()
            .filter(|p| self.registry.health().is_available(p.id))
            .map(|p| {
                let score = p.efficiency_score();
                RankedProtocol {
                    protocol: p,
                    score,
                    rank: 0,
                }
            })
            .collect();

        // Sort by score descending, with deterministic tiebreaker (by name)
        ranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.protocol.name.cmp(&b.protocol.name))
        });

        // Assign ranks
        for (i, r) in ranked.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        if ranked.is_empty() {
            return Err(HydraError::AllProtocolsFailed(
                "All matching protocols are unavailable".into(),
            ));
        }

        let primary = ranked.remove(0);
        Ok(DiscoveryResult {
            primary: Some(primary),
            fallbacks: ranked,
            manual_guidance: None,
        })
    }

    /// Discover with timeout (for sister-based protocols that may hang)
    pub async fn discover_with_timeout(
        &self,
        capability: &str,
    ) -> Result<DiscoveryResult, HydraError> {
        let timeout = self.timeout;
        let result = tokio::time::timeout(timeout, async { self.discover(capability) }).await;

        match result {
            Ok(inner) => inner,
            Err(_) => Err(HydraError::Timeout),
        }
    }

    /// Check version compatibility
    pub fn check_version(&self, protocol_id: uuid::Uuid, required_version: &str) -> bool {
        self.registry
            .get(protocol_id)
            .and_then(|p| p.version)
            .map(|v| v == required_version)
            .unwrap_or(false)
    }

    /// Negotiate version — returns the actual version if available
    pub fn negotiate_version(&self, protocol_id: uuid::Uuid) -> Option<String> {
        self.registry.get(protocol_id).and_then(|p| p.version)
    }
}
