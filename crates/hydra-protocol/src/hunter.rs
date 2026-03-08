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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProtocolKind;

    fn make_registry_with_entries() -> ProtocolRegistry {
        let reg = ProtocolRegistry::new();
        let entry1 = ProtocolEntry::new("memory-sister", ProtocolKind::Sister)
            .with_capabilities(vec!["memory"])
            .with_version("1.0.0");
        let entry2 = ProtocolEntry::new("memory-rest", ProtocolKind::RestApi)
            .with_capabilities(vec!["memory"])
            .with_version("2.0.0");
        let entry3 = ProtocolEntry::new("vision-sister", ProtocolKind::Sister)
            .with_capabilities(vec!["vision"]);
        reg.register(entry1);
        reg.register(entry2);
        reg.register(entry3);
        reg
    }

    #[test]
    fn test_hunter_discover_existing() {
        let reg = make_registry_with_entries();
        let hunter = ProtocolHunter::new(reg);
        let result = hunter.discover("memory").unwrap();
        assert!(result.primary.is_some());
    }

    #[test]
    fn test_hunter_discover_nonexistent() {
        let reg = make_registry_with_entries();
        let hunter = ProtocolHunter::new(reg);
        let result = hunter.discover("codebase").unwrap();
        assert!(result.is_empty());
        assert!(result.manual_guidance.is_some());
    }

    #[test]
    fn test_hunter_discover_ranks_by_efficiency() {
        let reg = make_registry_with_entries();
        let hunter = ProtocolHunter::new(reg);
        let result = hunter.discover("memory").unwrap();
        // Sister (100 tokens) should rank higher than RestApi (500 tokens)
        let primary = result.primary.unwrap();
        assert_eq!(primary.protocol.kind, ProtocolKind::Sister);
        assert_eq!(primary.rank, 1);
    }

    #[test]
    fn test_hunter_discover_has_fallbacks() {
        let reg = make_registry_with_entries();
        let hunter = ProtocolHunter::new(reg);
        let result = hunter.discover("memory").unwrap();
        assert!(!result.fallbacks.is_empty());
    }

    #[test]
    fn test_discovery_result_empty() {
        let result = DiscoveryResult::empty();
        assert!(result.is_empty());
        assert!(!result.is_fallback());
    }

    #[test]
    fn test_discovery_result_with_manual() {
        let result = DiscoveryResult::empty().with_manual("Try something else");
        assert_eq!(result.manual_guidance.as_deref(), Some("Try something else"));
    }

    #[test]
    fn test_hunter_check_version_match() {
        let reg = ProtocolRegistry::new();
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_version("1.0.0");
        let id = entry.id;
        reg.register(entry);
        let hunter = ProtocolHunter::new(reg);
        assert!(hunter.check_version(id, "1.0.0"));
    }

    #[test]
    fn test_hunter_check_version_mismatch() {
        let reg = ProtocolRegistry::new();
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_version("1.0.0");
        let id = entry.id;
        reg.register(entry);
        let hunter = ProtocolHunter::new(reg);
        assert!(!hunter.check_version(id, "2.0.0"));
    }

    #[test]
    fn test_hunter_check_version_nonexistent() {
        let reg = ProtocolRegistry::new();
        let hunter = ProtocolHunter::new(reg);
        assert!(!hunter.check_version(uuid::Uuid::new_v4(), "1.0.0"));
    }

    #[test]
    fn test_hunter_negotiate_version() {
        let reg = ProtocolRegistry::new();
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister)
            .with_version("3.0.0");
        let id = entry.id;
        reg.register(entry);
        let hunter = ProtocolHunter::new(reg);
        assert_eq!(hunter.negotiate_version(id), Some("3.0.0".into()));
    }

    #[test]
    fn test_hunter_negotiate_version_none() {
        let reg = ProtocolRegistry::new();
        let hunter = ProtocolHunter::new(reg);
        assert!(hunter.negotiate_version(uuid::Uuid::new_v4()).is_none());
    }

    #[test]
    fn test_hunter_set_timeout() {
        let reg = ProtocolRegistry::new();
        let mut hunter = ProtocolHunter::new(reg);
        hunter.set_timeout(Duration::from_secs(10));
        assert_eq!(hunter.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_hunter_registry_accessor() {
        let reg = make_registry_with_entries();
        let hunter = ProtocolHunter::new(reg);
        assert_eq!(hunter.registry().count(), 3);
    }

    #[test]
    fn test_ranked_protocol_score() {
        let entry = ProtocolEntry::new("test", ProtocolKind::Sister);
        let ranked = RankedProtocol {
            protocol: entry,
            score: 0.85,
            rank: 1,
        };
        assert_eq!(ranked.rank, 1);
        assert!((ranked.score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hunter_discover_circular_dependency_error() {
        let reg = ProtocolRegistry::new();
        let mut entry = ProtocolEntry::new("self-dep", ProtocolKind::Sister)
            .with_capabilities(vec!["test"]);
        entry.depends_on.push(entry.id);
        reg.register(entry);
        let hunter = ProtocolHunter::new(reg);
        let result = hunter.discover("test");
        assert!(result.is_err());
    }
}
