//! Obstacle resolution engine — detects, diagnoses, and resolves obstacles autonomously.
//!
//! When Hydra hits any error, the ObstacleResolver:
//! 1. Classifies the error into an ObstaclePattern
//! 2. Checks belief store for a known solution
//! 3. Uses multi-turn LLM to diagnose and generate fix strategies
//! 4. Applies fixes with checkpoints (reverts on failure)
//! 5. Verifies the fix worked
//! 6. Stores successful solutions as beliefs

pub mod detector;
pub mod diagnoser;
pub mod resolver;
pub mod verifier;

pub use detector::{Obstacle, ObstaclePattern};
pub use diagnoser::{Diagnosis, FixAction, RiskLevel, Strategy};
pub use resolver::{FileCheckpoint, Resolution, ResolverConfig, StoredSolution};
pub use verifier::VerifyResult;

use std::collections::HashMap;

/// The main obstacle resolution engine.
pub struct ObstacleResolver {
    config: ResolverConfig,
    /// Known solutions from belief store (obstacle_key → solution).
    known_solutions: HashMap<String, StoredSolution>,
    /// Stats for the current session.
    stats: ResolverStats,
}

#[derive(Debug, Default)]
pub struct ResolverStats {
    pub obstacles_seen: usize,
    pub auto_resolved: usize,
    pub from_memory: usize,
    pub escalated: usize,
}

impl ObstacleResolver {
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            config,
            known_solutions: HashMap::new(),
            stats: ResolverStats::default(),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(ResolverConfig::default())
    }

    /// Load known solutions from belief store subjects.
    pub fn load_beliefs(&mut self, beliefs: Vec<(String, String)>) {
        for (key, strategy_desc) in beliefs {
            self.known_solutions.insert(
                key.clone(),
                StoredSolution {
                    obstacle_key: key,
                    strategy: Strategy {
                        description: strategy_desc,
                        actions: Vec::new(),
                        risk_level: RiskLevel::Low,
                    },
                    times_used: 0,
                },
            );
        }
    }

    /// Check if we have a stored solution for this obstacle.
    pub fn lookup_known_solution(&self, obstacle: &Obstacle) -> Option<&StoredSolution> {
        self.known_solutions.get(&obstacle.belief_key())
    }

    /// Record a successful solution for future use.
    pub fn store_solution(&mut self, obstacle: &Obstacle, strategy: &Strategy) {
        let key = obstacle.belief_key();
        self.known_solutions.insert(
            key.clone(),
            StoredSolution {
                obstacle_key: key,
                strategy: strategy.clone(),
                times_used: 1,
            },
        );
    }

    /// Create a belief record for persistence.
    pub fn solution_as_belief(obstacle: &Obstacle, strategy: &Strategy) -> (String, String, String) {
        let subject = obstacle.belief_key();
        let content = format!(
            "Obstacle: {}\nPattern: {}\nFix: {}\nActions: {}",
            obstacle.error_message.lines().next().unwrap_or(""),
            obstacle.pattern.label(),
            strategy.description,
            strategy.actions.len(),
        );
        let category = "Correction".to_string();
        (subject, content, category)
    }

    /// The main resolution flow (sync version — LLM calls handled externally).
    ///
    /// Returns the resolution decision based on what we know.
    /// Callers provide the LLM-generated diagnosis and strategies.
    pub fn resolve_with_strategies(
        &mut self,
        obstacle: &Obstacle,
        diagnosis: Option<Diagnosis>,
        strategies: Vec<Strategy>,
    ) -> Resolution {
        self.stats.obstacles_seen += 1;

        // 1. Check for known solution
        if let Some(solution) = self.known_solutions.get_mut(&obstacle.belief_key()) {
            solution.times_used += 1;
            self.stats.from_memory += 1;
            return Resolution::FixedFromMemory {
                belief_key: obstacle.belief_key(),
            };
        }

        // 2. Check if pattern needs approval
        if !obstacle.pattern.is_auto_resolvable() {
            return Resolution::NeedsApproval {
                pattern: obstacle.pattern.clone(),
            };
        }

        // 3. No strategies available
        if strategies.is_empty() {
            self.stats.escalated += 1;
            return Resolution::Escalated {
                diagnosis: diagnosis
                    .map(|d| d.root_cause)
                    .unwrap_or_else(|| "No diagnosis available".to_string()),
                strategies_tried: 0,
            };
        }

        // 4. Rank and return the best strategy
        let ranked = resolver::rank_strategies(&strategies, obstacle);
        if let Some(&best_idx) = ranked.first() {
            let best = &strategies[best_idx];
            self.stats.auto_resolved += 1;
            Resolution::Fixed {
                attempts: 1,
                strategy_used: best.description.clone(),
            }
        } else {
            self.stats.escalated += 1;
            Resolution::Escalated {
                diagnosis: "No viable strategy found".to_string(),
                strategies_tried: strategies.len(),
            }
        }
    }

    /// Build LLM prompts for diagnosis and strategy generation.
    pub fn build_diagnosis_prompt(&self, obstacle: &Obstacle) -> String {
        diagnoser::build_diagnosis_prompt(obstacle)
    }

    pub fn build_strategy_prompt(&self, obstacle: &Obstacle, diagnosis: &Diagnosis) -> String {
        diagnoser::build_strategy_prompt(obstacle, diagnosis)
    }

    pub fn build_file_modify_prompt(
        &self,
        file_path: &str,
        file_content: &str,
        instruction: &str,
    ) -> String {
        diagnoser::build_file_modify_prompt(file_path, file_content, instruction)
    }

    /// Get current session stats.
    pub fn stats(&self) -> &ResolverStats {
        &self.stats
    }
}

/// Create an obstacle-resolution status message for the UI.
pub fn status_message(obstacle: &Obstacle, phase: &str) -> String {
    format!(
        "[Obstacle] {} detected: {} — {}",
        obstacle.pattern.label(),
        obstacle
            .error_message
            .lines()
            .next()
            .unwrap_or("unknown error"),
        phase,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_new() {
        let r = ObstacleResolver::with_default_config();
        assert_eq!(r.stats().obstacles_seen, 0);
    }

    #[test]
    fn test_lookup_no_solution() {
        let r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: unresolved import", "task");
        assert!(r.lookup_known_solution(&obs).is_none());
    }

    #[test]
    fn test_store_and_lookup() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: unresolved import", "task");
        let strategy = Strategy {
            description: "add import".into(),
            actions: vec![],
            risk_level: RiskLevel::Low,
        };
        r.store_solution(&obs, &strategy);
        assert!(r.lookup_known_solution(&obs).is_some());
    }

    #[test]
    fn test_resolve_from_memory() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: unresolved import", "task");
        let strategy = Strategy {
            description: "add import".into(),
            actions: vec![],
            risk_level: RiskLevel::Low,
        };
        r.store_solution(&obs, &strategy);

        let result = r.resolve_with_strategies(&obs, None, vec![]);
        assert!(result.is_fixed());
        assert_eq!(r.stats().from_memory, 1);
    }

    #[test]
    fn test_resolve_needs_approval() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("permission denied: /etc/passwd", "task");
        let result = r.resolve_with_strategies(&obs, None, vec![]);
        assert!(matches!(result, Resolution::NeedsApproval { .. }));
    }

    #[test]
    fn test_resolve_no_strategies_escalates() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: something", "task");
        let diag = Diagnosis {
            root_cause: "missing import".into(),
            affected_files: vec![],
            suggested_approach: "add it".into(),
            confidence: 0.8,
        };
        let result = r.resolve_with_strategies(&obs, Some(diag), vec![]);
        assert!(matches!(result, Resolution::Escalated { .. }));
        assert_eq!(r.stats().escalated, 1);
    }

    #[test]
    fn test_resolve_with_strategy() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: unresolved", "task");
        let strategies = vec![Strategy {
            description: "fix import".into(),
            actions: vec![FixAction::ModifyFile {
                path: "src/lib.rs".into(),
                instruction: "add use".into(),
            }],
            risk_level: RiskLevel::Low,
        }];
        let result = r.resolve_with_strategies(&obs, None, strategies);
        assert!(result.is_fixed());
        assert_eq!(r.stats().auto_resolved, 1);
    }

    #[test]
    fn test_solution_as_belief() {
        let obs = Obstacle::from_error("error[E0433]", "task");
        let strategy = Strategy {
            description: "add import".into(),
            actions: vec![],
            risk_level: RiskLevel::Low,
        };
        let (subject, content, category) = ObstacleResolver::solution_as_belief(&obs, &strategy);
        assert!(subject.starts_with("obstacle:"));
        assert!(content.contains("add import"));
        assert_eq!(category, "Correction");
    }

    #[test]
    fn test_load_beliefs() {
        let mut r = ObstacleResolver::with_default_config();
        let obs = Obstacle::from_error("error[E0433]: test", "task");
        let key = obs.belief_key();
        r.load_beliefs(vec![(key, "fix it".into())]);
        assert!(r.lookup_known_solution(&obs).is_some());
    }

    #[test]
    fn test_status_message() {
        let obs = Obstacle::from_error("error[E0433]: bad import", "task");
        let msg = status_message(&obs, "diagnosing");
        assert!(msg.contains("Compilation Error"));
        assert!(msg.contains("diagnosing"));
    }
}
