//! PatternMutator — create variations of action patterns.

use serde::{Deserialize, Serialize};

use super::tracker::PatternRecord;

/// Type of mutation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    /// Reorder steps
    Reorder,
    /// Remove a step
    RemoveStep,
    /// Add a step
    AddStep,
    /// Replace a step with alternative
    ReplaceStep,
    /// Merge consecutive steps
    MergeSteps,
}

/// A mutation applied to a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub id: String,
    pub source_pattern: String,
    pub mutation_type: MutationType,
    pub description: String,
    pub new_actions: Vec<String>,
}

/// Generates mutations of action patterns
pub struct PatternMutator {
    mutations: parking_lot::RwLock<Vec<Mutation>>,
}

impl PatternMutator {
    pub fn new() -> Self {
        Self {
            mutations: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Generate mutations for a pattern
    pub fn mutate(&self, pattern: &PatternRecord) -> Vec<Mutation> {
        let mut mutations = Vec::new();

        // Reorder: swap adjacent steps
        if pattern.actions.len() >= 2 {
            let mut reordered = pattern.actions.clone();
            reordered.swap(0, 1);
            mutations.push(Mutation {
                id: uuid::Uuid::new_v4().to_string(),
                source_pattern: pattern.id.clone(),
                mutation_type: MutationType::Reorder,
                description: format!("Swap '{}' and '{}'", pattern.actions[0], pattern.actions[1]),
                new_actions: reordered,
            });
        }

        // Remove: try removing each non-essential step
        if pattern.actions.len() > 1 {
            for i in 0..pattern.actions.len() {
                let mut removed = pattern.actions.clone();
                let removed_action = removed.remove(i);
                mutations.push(Mutation {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_pattern: pattern.id.clone(),
                    mutation_type: MutationType::RemoveStep,
                    description: format!("Remove step '{}'", removed_action),
                    new_actions: removed,
                });
            }
        }

        // Add: add validation step
        {
            let mut with_validation = pattern.actions.clone();
            with_validation.push("validate".into());
            mutations.push(Mutation {
                id: uuid::Uuid::new_v4().to_string(),
                source_pattern: pattern.id.clone(),
                mutation_type: MutationType::AddStep,
                description: "Add validation step at end".into(),
                new_actions: with_validation,
            });
        }

        self.mutations.write().extend(mutations.clone());
        mutations
    }

    /// Get all generated mutations
    pub fn all_mutations(&self) -> Vec<Mutation> {
        self.mutations.read().clone()
    }

    pub fn mutation_count(&self) -> usize {
        self.mutations.read().len()
    }
}

impl Default for PatternMutator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_mutation() {
        let mutator = PatternMutator::new();
        let pattern = PatternRecord::new(
            "edit_flow",
            vec!["read".into(), "modify".into(), "write".into()],
        );

        let mutations = mutator.mutate(&pattern);
        assert!(!mutations.is_empty());
        // Should have: 1 reorder + 3 remove + 1 add = 5
        assert_eq!(mutations.len(), 5);
    }

    #[test]
    fn test_mutation_variants() {
        let mutator = PatternMutator::new();
        let pattern = PatternRecord::new("simple", vec!["step_a".into(), "step_b".into()]);

        let mutations = mutator.mutate(&pattern);

        let types: Vec<_> = mutations.iter().map(|m| &m.mutation_type).collect();
        assert!(types.contains(&&MutationType::Reorder));
        assert!(types.contains(&&MutationType::RemoveStep));
        assert!(types.contains(&&MutationType::AddStep));
    }
}
