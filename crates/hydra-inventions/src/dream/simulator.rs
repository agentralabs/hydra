//! DreamSimulator — background simulation during idle time.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::insights::{DreamInsight, InsightCategory, InsightStore};

/// Idle level determines what dreams can run
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IdleLevel {
    Active,
    LightIdle,
    DeepIdle,
    Sleeping,
}

/// Dream configuration
#[derive(Debug, Clone)]
pub struct DreamConfig {
    pub enabled: bool,
    pub max_dream_duration: Duration,
    pub min_idle_level: IdleLevel,
    pub max_resource_pct: f32,
    pub max_insights_per_session: usize,
}

impl Default for DreamConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_dream_duration: Duration::from_secs(60),
            min_idle_level: IdleLevel::LightIdle,
            max_resource_pct: 0.3,
            max_insights_per_session: 10,
        }
    }
}

/// A dream task to execute during idle
#[derive(Debug, Clone)]
pub struct DreamTask {
    pub name: String,
    pub priority: u8,
    pub min_idle_level: IdleLevel,
    pub category: InsightCategory,
    pub description: String,
}

/// Dream simulator that runs background exploration
pub struct DreamSimulator {
    config: DreamConfig,
    current_idle: parking_lot::Mutex<IdleLevel>,
    insights: InsightStore,
    tasks: Vec<DreamTask>,
    session_count: parking_lot::Mutex<u64>,
}

impl DreamSimulator {
    pub fn new(config: DreamConfig) -> Self {
        Self {
            config,
            current_idle: parking_lot::Mutex::new(IdleLevel::Active),
            insights: InsightStore::new(100),
            tasks: default_dream_tasks(),
            session_count: parking_lot::Mutex::new(0),
        }
    }

    /// Update the current idle level
    pub fn set_idle_level(&self, level: IdleLevel) {
        *self.current_idle.lock() = level;
    }

    /// Get current idle level
    pub fn idle_level(&self) -> IdleLevel {
        *self.current_idle.lock()
    }

    /// Check if dreaming is possible at current idle level
    pub fn can_dream(&self) -> bool {
        self.config.enabled && self.idle_level() >= self.config.min_idle_level
    }

    /// Get eligible dream tasks for current idle level
    pub fn eligible_tasks(&self) -> Vec<&DreamTask> {
        let level = self.idle_level();
        let mut tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|t| level >= t.min_idle_level)
            .collect();
        tasks.sort_by_key(|t| t.priority);
        tasks
    }

    /// Run a dream session (simulated — real implementation would call LLM)
    pub fn dream_session(&self) -> Vec<DreamInsight> {
        if !self.can_dream() {
            return Vec::new();
        }

        let eligible = self.eligible_tasks();
        let mut session_insights = Vec::new();

        for task in eligible.iter().take(self.config.max_insights_per_session) {
            let insight = DreamInsight::new(
                &task.name,
                task.category,
                &format!("Dream insight from {}: {}", task.name, task.description),
                0.5 + (task.priority as f32 * 0.05),
            );
            self.insights.add(insight.clone());
            session_insights.push(insight);
        }

        *self.session_count.lock() += 1;
        session_insights
    }

    /// Get all stored insights
    pub fn insights(&self) -> &InsightStore {
        &self.insights
    }

    /// Number of dream sessions run
    pub fn session_count(&self) -> u64 {
        *self.session_count.lock()
    }
}

fn default_dream_tasks() -> Vec<DreamTask> {
    vec![
        DreamTask {
            name: "pattern_mining".into(),
            priority: 1,
            min_idle_level: IdleLevel::LightIdle,
            category: InsightCategory::PatternDiscovered,
            description: "Mine action patterns from recent receipts".into(),
        },
        DreamTask {
            name: "memory_consolidation".into(),
            priority: 2,
            min_idle_level: IdleLevel::DeepIdle,
            category: InsightCategory::MemoryConsolidated,
            description: "Strengthen frequent memories, decay stale ones".into(),
        },
        DreamTask {
            name: "context_prefetch".into(),
            priority: 3,
            min_idle_level: IdleLevel::LightIdle,
            category: InsightCategory::PredictionCached,
            description: "Pre-compute likely morning queries".into(),
        },
        DreamTask {
            name: "optimization_scan".into(),
            priority: 4,
            min_idle_level: IdleLevel::DeepIdle,
            category: InsightCategory::OptimizationFound,
            description: "Find optimization opportunities in workflows".into(),
        },
        DreamTask {
            name: "counterfactual".into(),
            priority: 5,
            min_idle_level: IdleLevel::DeepIdle,
            category: InsightCategory::CounterfactualExplored,
            description: "Explore what-if scenarios".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_trigger() {
        let sim = DreamSimulator::new(DreamConfig::default());
        assert!(!sim.can_dream()); // Active, can't dream

        sim.set_idle_level(IdleLevel::LightIdle);
        assert!(sim.can_dream());
    }

    #[test]
    fn test_dream_scenario_gen() {
        let sim = DreamSimulator::new(DreamConfig::default());
        sim.set_idle_level(IdleLevel::LightIdle);

        let eligible = sim.eligible_tasks();
        assert!(!eligible.is_empty());
        // LightIdle: pattern_mining, context_prefetch eligible
        assert!(eligible.iter().any(|t| t.name == "pattern_mining"));
    }

    #[test]
    fn test_dream_prioritization() {
        let sim = DreamSimulator::new(DreamConfig::default());
        sim.set_idle_level(IdleLevel::DeepIdle);

        let eligible = sim.eligible_tasks();
        // All tasks eligible at DeepIdle, sorted by priority
        assert_eq!(eligible[0].priority, 1);
        assert_eq!(eligible.last().unwrap().priority, 5);
    }

    #[test]
    fn test_dream_storage() {
        let sim = DreamSimulator::new(DreamConfig::default());
        sim.set_idle_level(IdleLevel::DeepIdle);

        let insights = sim.dream_session();
        assert!(!insights.is_empty());
        assert_eq!(sim.session_count(), 1);
        assert_eq!(sim.insights().count(), insights.len());
    }

    #[test]
    fn test_resource_limits() {
        let config = DreamConfig {
            max_insights_per_session: 2,
            ..Default::default()
        };
        let sim = DreamSimulator::new(config);
        sim.set_idle_level(IdleLevel::DeepIdle);

        let insights = sim.dream_session();
        assert!(insights.len() <= 2);
    }
}
