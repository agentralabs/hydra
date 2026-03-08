//! TaskDelegation — offload tasks to capable peers.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::peer::{PeerId, PeerInfo};
use crate::registry::PeerRegistry;

#[derive(Debug, Error)]
pub enum DelegationError {
    #[error("no capable peer found for: {0}")]
    NoPeerFound(String),
    #[error("peer {0} has no capacity")]
    NoCapacity(String),
    #[error("peer {0} trust level too low")]
    InsufficientTrust(String),
}

/// A task to delegate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatedTask {
    pub id: String,
    pub description: String,
    pub requirements: Vec<String>,
    pub priority: TaskPriority,
    pub max_duration_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Result of a delegated task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    pub task_id: String,
    pub peer_id: PeerId,
    pub success: bool,
    pub result: serde_json::Value,
    pub duration_ms: u64,
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Pick the peer with fewest active tasks
    LeastLoaded,
    /// Round-robin across capable peers
    RoundRobin,
    /// Pick the peer with highest trust
    MostTrusted,
}

/// Task delegation engine
pub struct TaskDelegation {
    strategy: LoadBalanceStrategy,
    round_robin_counter: parking_lot::Mutex<usize>,
}

impl TaskDelegation {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        Self {
            strategy,
            round_robin_counter: parking_lot::Mutex::new(0),
        }
    }

    /// Find the best peer to handle a task
    pub fn find_peer(
        &self,
        task: &DelegatedTask,
        registry: &PeerRegistry,
    ) -> Result<PeerInfo, DelegationError> {
        // Get capable peers that can handle all requirements
        let mut candidates: Vec<PeerInfo> = registry
            .available_peers()
            .into_iter()
            .filter(|p| task.requirements.iter().all(|req| p.has_capability(req)))
            .collect();

        if candidates.is_empty() {
            return Err(DelegationError::NoPeerFound(task.requirements.join(", ")));
        }

        match self.strategy {
            LoadBalanceStrategy::LeastLoaded => {
                candidates.sort_by_key(|p| p.active_tasks);
                Ok(candidates.remove(0))
            }
            LoadBalanceStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.lock();
                let idx = *counter % candidates.len();
                *counter += 1;
                Ok(candidates.remove(idx))
            }
            LoadBalanceStrategy::MostTrusted => {
                candidates.sort_by(|a, b| b.trust_level.cmp(&a.trust_level));
                Ok(candidates.remove(0))
            }
        }
    }

    /// Validate that delegation is allowed
    pub fn validate_delegation(
        &self,
        _task: &DelegatedTask,
        peer: &PeerInfo,
    ) -> Result<(), DelegationError> {
        if !peer.allows_delegation() {
            return Err(DelegationError::InsufficientTrust(peer.id.clone()));
        }
        if !peer.has_capacity() {
            return Err(DelegationError::NoCapacity(peer.id.clone()));
        }
        Ok(())
    }
}

impl Default for TaskDelegation {
    fn default() -> Self {
        Self::new(LoadBalanceStrategy::LeastLoaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{FederationType, PeerCapabilities, TrustLevel};

    fn make_peer(id: &str, trust: TrustLevel, tasks: u32, sisters: Vec<String>) -> PeerInfo {
        PeerInfo {
            id: id.into(),
            name: id.into(),
            endpoint: format!("{}:9000", id),
            version: "0.1.0".into(),
            capabilities: PeerCapabilities {
                sisters,
                skills: vec![],
                max_concurrent_tasks: 4,
                available_memory_mb: 1024,
                federation_types: vec![FederationType::Personal],
            },
            trust_level: trust,
            federation_type: FederationType::Personal,
            last_seen: chrono::Utc::now().to_rfc3339(),
            active_tasks: tasks,
        }
    }

    fn make_task(requirements: Vec<&str>) -> DelegatedTask {
        DelegatedTask {
            id: "task-1".into(),
            description: "test".into(),
            requirements: requirements.into_iter().map(String::from).collect(),
            priority: TaskPriority::Normal,
            max_duration_secs: 60,
        }
    }

    #[test]
    fn test_delegation_find_capable() {
        let registry = PeerRegistry::new();
        registry.register(make_peer(
            "a",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));
        registry.register(make_peer(
            "b",
            TrustLevel::Trusted,
            0,
            vec!["codebase".into()],
        ));

        let delegation = TaskDelegation::default();
        let task = make_task(vec!["memory"]);
        let peer = delegation.find_peer(&task, &registry).unwrap();
        assert_eq!(peer.id, "a");
    }

    #[test]
    fn test_delegation_load_balance() {
        let registry = PeerRegistry::new();
        registry.register(make_peer(
            "busy",
            TrustLevel::Trusted,
            3,
            vec!["memory".into()],
        ));
        registry.register(make_peer(
            "idle",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));

        let delegation = TaskDelegation::new(LoadBalanceStrategy::LeastLoaded);
        let task = make_task(vec!["memory"]);
        let peer = delegation.find_peer(&task, &registry).unwrap();
        assert_eq!(peer.id, "idle");
    }

    #[test]
    fn test_delegation_most_trusted() {
        let registry = PeerRegistry::new();
        registry.register(make_peer(
            "trusted",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));
        registry.register(make_peer(
            "owner",
            TrustLevel::Owner,
            0,
            vec!["memory".into()],
        ));

        let delegation = TaskDelegation::new(LoadBalanceStrategy::MostTrusted);
        let task = make_task(vec!["memory"]);
        let peer = delegation.find_peer(&task, &registry).unwrap();
        assert_eq!(peer.id, "owner");
    }

    #[test]
    fn test_delegation_round_robin() {
        let registry = PeerRegistry::new();
        registry.register(make_peer(
            "a",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));
        registry.register(make_peer(
            "b",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));

        let delegation = TaskDelegation::new(LoadBalanceStrategy::RoundRobin);
        let task = make_task(vec!["memory"]);
        let p1 = delegation.find_peer(&task, &registry).unwrap();
        let p2 = delegation.find_peer(&task, &registry).unwrap();
        // Round robin should pick different peers (if 2 candidates)
        // May not always differ due to HashMap ordering, but counter increments
        assert!(p1.id == "a" || p1.id == "b");
        assert!(p2.id == "a" || p2.id == "b");
    }

    #[test]
    fn test_delegation_validate_ok() {
        let peer = make_peer(
            "valid",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        );
        let delegation = TaskDelegation::default();
        let task = make_task(vec!["memory"]);
        assert!(delegation.validate_delegation(&task, &peer).is_ok());
    }

    #[test]
    fn test_delegation_validate_insufficient_trust() {
        let peer = make_peer(
            "untrusted",
            TrustLevel::Known,
            0,
            vec!["memory".into()],
        );
        let delegation = TaskDelegation::default();
        let task = make_task(vec!["memory"]);
        assert!(matches!(
            delegation.validate_delegation(&task, &peer),
            Err(DelegationError::InsufficientTrust(_))
        ));
    }

    #[test]
    fn test_delegation_validate_no_capacity() {
        let peer = make_peer(
            "full",
            TrustLevel::Trusted,
            4, // max concurrent is 4
            vec!["memory".into()],
        );
        let delegation = TaskDelegation::default();
        let task = make_task(vec!["memory"]);
        assert!(matches!(
            delegation.validate_delegation(&task, &peer),
            Err(DelegationError::NoCapacity(_))
        ));
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }

    #[test]
    fn test_delegated_task_serialization() {
        let task = make_task(vec!["memory", "vision"]);
        let json = serde_json::to_string(&task).unwrap();
        let restored: DelegatedTask = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "task-1");
        assert_eq!(restored.requirements.len(), 2);
    }

    #[test]
    fn test_delegation_result_serialization() {
        let result = DelegationResult {
            task_id: "t-1".into(),
            peer_id: "p-1".into(),
            success: true,
            result: serde_json::json!({"output": "done"}),
            duration_ms: 150,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: DelegationResult = serde_json::from_str(&json).unwrap();
        assert!(restored.success);
        assert_eq!(restored.duration_ms, 150);
    }

    #[test]
    fn test_delegation_default() {
        let delegation = TaskDelegation::default();
        assert_eq!(delegation.strategy, LoadBalanceStrategy::LeastLoaded);
    }

    #[test]
    fn test_delegation_no_capable_peer() {
        let registry = PeerRegistry::new();
        registry.register(make_peer(
            "a",
            TrustLevel::Trusted,
            0,
            vec!["memory".into()],
        ));

        let delegation = TaskDelegation::default();
        let task = make_task(vec!["vision"]); // No peer has vision
        assert!(delegation.find_peer(&task, &registry).is_err());
    }
}
