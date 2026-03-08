//! EvolutionEngine — selection and evolution of patterns toward optimality.

use serde::{Deserialize, Serialize};

use super::tracker::PatternRecord;

/// A generation of pattern variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub number: u32,
    pub patterns: Vec<PatternRecord>,
    pub best_fitness: f64,
    pub avg_fitness: f64,
}

/// Evolution engine that selects and evolves patterns
pub struct EvolutionEngine {
    generations: parking_lot::RwLock<Vec<Generation>>,
    selection_pressure: f64,
    min_fitness: f64,
}

impl EvolutionEngine {
    pub fn new(selection_pressure: f64) -> Self {
        Self {
            generations: parking_lot::RwLock::new(Vec::new()),
            selection_pressure: selection_pressure.clamp(0.1, 1.0),
            min_fitness: 0.3,
        }
    }

    /// Calculate fitness of a pattern (success rate weighted by execution count)
    pub fn fitness(&self, pattern: &PatternRecord) -> f64 {
        let rate = pattern.success_rate();
        let experience = (pattern.total_executions as f64).ln().max(0.0) + 1.0;
        let speed = if pattern.avg_duration_ms > 0.0 {
            1000.0 / pattern.avg_duration_ms
        } else {
            1.0
        };
        // Weighted: 60% success rate, 30% experience, 10% speed
        rate * 0.6 + (experience / 10.0).min(0.3) + (speed / 100.0).min(0.1)
    }

    /// Select the fittest patterns (tournament selection)
    pub fn select(&self, patterns: &[PatternRecord]) -> Vec<PatternRecord> {
        let mut scored: Vec<_> = patterns
            .iter()
            .map(|p| (self.fitness(p), p.clone()))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let keep = ((patterns.len() as f64) * self.selection_pressure).max(1.0) as usize;
        scored
            .into_iter()
            .take(keep)
            .filter(|(f, _)| *f >= self.min_fitness)
            .map(|(_, p)| p)
            .collect()
    }

    /// Run one generation of evolution
    pub fn evolve(&self, patterns: Vec<PatternRecord>) -> Generation {
        let selected = self.select(&patterns);

        let best_fitness = selected
            .iter()
            .map(|p| self.fitness(p))
            .fold(0.0f64, f64::max);

        let avg_fitness = if !selected.is_empty() {
            selected.iter().map(|p| self.fitness(p)).sum::<f64>() / selected.len() as f64
        } else {
            0.0
        };

        let gen_number = self.generations.read().len() as u32 + 1;

        let generation = Generation {
            number: gen_number,
            patterns: selected,
            best_fitness,
            avg_fitness,
        };

        self.generations.write().push(generation.clone());
        generation
    }

    /// Get generation history
    pub fn history(&self) -> Vec<Generation> {
        self.generations.read().clone()
    }

    /// Check if convergence has been reached
    pub fn converged(&self, threshold: f64) -> bool {
        let gens = self.generations.read();
        if gens.len() < 3 {
            return false;
        }
        let last_three: Vec<f64> = gens.iter().rev().take(3).map(|g| g.best_fitness).collect();
        let range = last_three.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - last_three.iter().cloned().fold(f64::INFINITY, f64::min);
        range < threshold
    }

    pub fn generation_count(&self) -> usize {
        self.generations.read().len()
    }
}

impl Default for EvolutionEngine {
    fn default() -> Self {
        Self::new(0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pattern(name: &str, successes: u64, failures: u64) -> PatternRecord {
        let mut p = PatternRecord::new(name, vec![name.into()]);
        for _ in 0..successes {
            p.record_execution(true, 100.0);
        }
        for _ in 0..failures {
            p.record_execution(false, 100.0);
        }
        p
    }

    #[test]
    fn test_evolution_selection() {
        let engine = EvolutionEngine::new(0.5);
        let patterns = vec![
            make_pattern("good", 9, 1),  // 90% success
            make_pattern("ok", 6, 4),    // 60% success
            make_pattern("bad", 2, 8),   // 20% success
            make_pattern("worse", 1, 9), // 10% success
        ];

        let selected = engine.select(&patterns);
        // Top 50% selected (2), but "worse" may be filtered by min_fitness
        assert!(selected.len() <= 2);
        assert!(
            selected[0].success_rate() > selected.last().unwrap().success_rate()
                || selected.len() == 1
        );
    }

    #[test]
    fn test_optimal_convergence() {
        let engine = EvolutionEngine::new(0.5);

        // Run identical generations to simulate convergence
        let patterns = vec![make_pattern("stable", 8, 2)];
        engine.evolve(patterns.clone());
        engine.evolve(patterns.clone());
        engine.evolve(patterns);

        assert!(engine.converged(0.01));
    }
}
