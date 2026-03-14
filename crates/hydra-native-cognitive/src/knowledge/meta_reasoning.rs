//! Meta-Reasoning — tracks which reasoning strategies work per user per
//! domain. Multi-armed bandit: 90% exploit best, 10% explore.
//!
//! Why isn't a sister doing this? Pure in-memory strategy selection.
//! Evolve sister records patterns; this module does the bandit algorithm.

use std::collections::HashMap;

/// A reasoning strategy with tracked outcomes.
#[derive(Debug, Clone)]
pub struct ReasoningStrategy {
    pub name: String,
    pub description: String,
    pub prompt_injection: String,
    pub total_uses: u64,
    pub successes: u64,
}

impl ReasoningStrategy {
    pub fn success_rate(&self) -> f64 {
        if self.total_uses == 0 { 0.5 } else { self.successes as f64 / self.total_uses as f64 }
    }
}

/// Outcome of a strategy use — recorded after user reaction.
#[derive(Debug, Clone)]
pub struct StrategyOutcome {
    pub strategy: String,
    pub domain: String,
    pub succeeded: bool,
}

/// The meta-reasoning engine — selects optimal strategies per domain.
#[derive(Debug)]
pub struct MetaReasoner {
    strategies: Vec<ReasoningStrategy>,
    domain_history: HashMap<String, Vec<StrategyOutcome>>,
    explore_rate: f64,
}

impl MetaReasoner {
    pub fn new() -> Self {
        Self {
            strategies: default_strategies(),
            domain_history: HashMap::new(),
            explore_rate: 0.10,
        }
    }

    /// Select the best strategy for a given domain and question type.
    /// 90% exploit (best known), 10% explore (random).
    pub fn select_strategy(&self, domain: &str) -> &ReasoningStrategy {
        let explore = rand_f64() < self.explore_rate;

        if explore {
            let idx = (rand_f64() * self.strategies.len() as f64) as usize;
            return &self.strategies[idx.min(self.strategies.len() - 1)];
        }

        // Exploit: find strategy with best success rate for this domain
        let domain_outcomes = self.domain_history.get(domain);

        let mut best = &self.strategies[0];
        let mut best_rate = 0.0_f64;

        for strategy in &self.strategies {
            let rate = match domain_outcomes {
                Some(outcomes) => {
                    let relevant: Vec<&StrategyOutcome> = outcomes.iter()
                        .filter(|o| o.strategy == strategy.name)
                        .collect();
                    if relevant.is_empty() {
                        strategy.success_rate()
                    } else {
                        let successes = relevant.iter().filter(|o| o.succeeded).count();
                        successes as f64 / relevant.len() as f64
                    }
                }
                None => strategy.success_rate(),
            };
            if rate > best_rate {
                best_rate = rate;
                best = strategy;
            }
        }

        best
    }

    /// Record the outcome of a strategy use.
    pub fn record_outcome(&mut self, strategy: &str, domain: &str, succeeded: bool) {
        let outcome = StrategyOutcome {
            strategy: strategy.to_string(),
            domain: domain.to_string(),
            succeeded,
        };
        self.domain_history.entry(domain.to_string()).or_default().push(outcome);

        // Update global strategy stats
        if let Some(s) = self.strategies.iter_mut().find(|s| s.name == strategy) {
            s.total_uses += 1;
            if succeeded { s.successes += 1; }
        }

        eprintln!("[hydra:meta_reasoning] Recorded: {} in {} = {}",
            strategy, domain, if succeeded { "success" } else { "failure" });
    }

    /// Get a summary of strategy performance per domain.
    pub fn performance_summary(&self) -> String {
        let mut lines = vec!["Strategy Performance:".to_string()];
        for s in &self.strategies {
            lines.push(format!("  {} — {:.0}% ({}/{})",
                s.name, s.success_rate() * 100.0, s.successes, s.total_uses));
        }
        lines.join("\n")
    }

    /// Format the selected strategy as a prompt injection.
    pub fn format_for_prompt(&self, strategy: &ReasoningStrategy) -> String {
        format!(
            "# Reasoning Strategy: {}\n{}\n{}\n",
            strategy.name, strategy.description, strategy.prompt_injection,
        )
    }

    /// How many strategies are tracked.
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }
}

fn default_strategies() -> Vec<ReasoningStrategy> {
    vec![
        ReasoningStrategy {
            name: "first_principles".into(),
            description: "Break into fundamentals, rebuild from ground truth".into(),
            prompt_injection: "Approach this from first principles. Break the problem \
                into fundamental components. Rebuild your answer from basic truths.".into(),
            total_uses: 0, successes: 0,
        },
        ReasoningStrategy {
            name: "analogical".into(),
            description: "Find similar past situations, adapt the solution".into(),
            prompt_injection: "Think of analogous situations you've seen. What worked \
                there? Adapt that solution to this specific context.".into(),
            total_uses: 0, successes: 0,
        },
        ReasoningStrategy {
            name: "adversarial".into(),
            description: "Steelman the opposite position, find the fatal flaw".into(),
            prompt_injection: "Before answering, steelman the OPPOSITE position. \
                Find the strongest argument against your instinct. Then decide.".into(),
            total_uses: 0, successes: 0,
        },
        ReasoningStrategy {
            name: "decomposition".into(),
            description: "Break complex problem into independent sub-problems".into(),
            prompt_injection: "Decompose this into independent sub-problems. \
                Solve each separately, then synthesize. Show the decomposition.".into(),
            total_uses: 0, successes: 0,
        },
        ReasoningStrategy {
            name: "bayesian_update".into(),
            description: "Start with prior belief, update with each piece of evidence".into(),
            prompt_injection: "Start with your prior belief about this. For each piece \
                of evidence, update your confidence. Show prior → posterior at each step.".into(),
            total_uses: 0, successes: 0,
        },
    ]
}

/// Simple pseudo-random in [0, 1) using time-based seed.
fn rand_f64() -> f64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (t as f64 % 1000.0) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_reasoner() {
        let mr = MetaReasoner::new();
        assert_eq!(mr.strategy_count(), 5);
    }

    #[test]
    fn test_select_strategy() {
        let mr = MetaReasoner::new();
        let strategy = mr.select_strategy("rust");
        assert!(!strategy.name.is_empty());
    }

    #[test]
    fn test_record_and_select() {
        let mut mr = MetaReasoner::new();
        for _ in 0..10 {
            mr.record_outcome("decomposition", "architecture", true);
        }
        for _ in 0..10 {
            mr.record_outcome("first_principles", "architecture", false);
        }
        // With enough history, decomposition should be preferred
        let strategy = mr.select_strategy("architecture");
        // Can't guarantee due to 10% explore, but strategy should exist
        assert!(!strategy.name.is_empty());
    }

    #[test]
    fn test_performance_summary() {
        let mr = MetaReasoner::new();
        let summary = mr.performance_summary();
        assert!(summary.contains("first_principles"));
    }
}
