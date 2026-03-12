//! SkillCrystallizer — extract reusable skills from observed patterns.

use serde::{Deserialize, Serialize};

use super::skill::{CrystallizedSkill, SkillComplexity, SkillTrigger};

/// Result of a crystallization attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationResult {
    pub skill_id: String,
    pub skill_name: String,
    pub success: bool,
    pub reason: String,
}

/// Source pattern data for crystallization
#[derive(Debug, Clone)]
pub struct PatternObservation {
    pub name: String,
    pub actions: Vec<String>,
    pub occurrences: u64,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
}

/// Crystallizes observed patterns into reusable skills
pub struct SkillCrystallizer {
    skills: parking_lot::RwLock<Vec<CrystallizedSkill>>,
    min_occurrences: u64,
    min_success_rate: f64,
}

impl SkillCrystallizer {
    pub fn new(min_occurrences: u64, min_success_rate: f64) -> Self {
        Self {
            skills: parking_lot::RwLock::new(Vec::new()),
            min_occurrences,
            min_success_rate,
        }
    }

    /// Attempt to crystallize a pattern into a skill
    pub fn crystallize(&self, observation: &PatternObservation) -> CrystallizationResult {
        // Check if pattern meets crystallization threshold
        if observation.occurrences < self.min_occurrences {
            return CrystallizationResult {
                skill_id: String::new(),
                skill_name: observation.name.clone(),
                success: false,
                reason: format!(
                    "Not enough observations ({}/{})",
                    observation.occurrences, self.min_occurrences
                ),
            };
        }

        if observation.success_rate < self.min_success_rate {
            return CrystallizationResult {
                skill_id: String::new(),
                skill_name: observation.name.clone(),
                success: false,
                reason: format!(
                    "Success rate too low ({:.0}%/{:.0}%)",
                    observation.success_rate * 100.0,
                    self.min_success_rate * 100.0,
                ),
            };
        }

        // Check for duplicate
        if self.skills.read().iter().any(|s| s.name == observation.name) {
            return CrystallizationResult {
                skill_id: String::new(),
                skill_name: observation.name.clone(),
                success: false,
                reason: "Skill already crystallized".into(),
            };
        }

        let complexity = match observation.actions.len() {
            1 => SkillComplexity::Atomic,
            2..=4 => SkillComplexity::Composite,
            _ => SkillComplexity::Complex,
        };

        let skill = CrystallizedSkill {
            id: uuid::Uuid::new_v4().to_string(),
            name: observation.name.clone(),
            description: format!(
                "Crystallized from {} observations with {:.0}% success",
                observation.occurrences,
                observation.success_rate * 100.0,
            ),
            actions: observation.actions.clone(),
            trigger: SkillTrigger::Manual,
            complexity,
            source_pattern: observation.name.clone(),
            confidence: observation.success_rate,
            executions: 0,
            avg_duration_ms: observation.avg_duration_ms,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = CrystallizationResult {
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            success: true,
            reason: "Pattern crystallized into skill".into(),
        };

        self.skills.write().push(skill);
        result
    }

    /// Get all crystallized skills
    pub fn skills(&self) -> Vec<CrystallizedSkill> {
        self.skills.read().clone()
    }

    /// Find a skill by name
    pub fn find_skill(&self, name: &str) -> Option<CrystallizedSkill> {
        self.skills.read().iter().find(|s| s.name == name).cloned()
    }

    /// Get the number of crystallized skills
    pub fn skill_count(&self) -> usize {
        self.skills.read().len()
    }
}

impl Default for SkillCrystallizer {
    fn default() -> Self {
        Self::new(3, 0.7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_observation() -> PatternObservation {
        PatternObservation {
            name: "file_edit".into(),
            actions: vec!["read".into(), "modify".into(), "write".into()],
            occurrences: 10,
            success_rate: 0.9,
            avg_duration_ms: 150.0,
        }
    }

    #[test]
    fn test_successful_crystallization() {
        let crystallizer = SkillCrystallizer::default();
        let result = crystallizer.crystallize(&good_observation());
        assert!(result.success);
        assert_eq!(crystallizer.skill_count(), 1);

        let skill = crystallizer.find_skill("file_edit").unwrap();
        assert_eq!(skill.actions.len(), 3);
        assert_eq!(skill.complexity, SkillComplexity::Composite);
    }

    #[test]
    fn test_insufficient_observations() {
        let crystallizer = SkillCrystallizer::default();
        let obs = PatternObservation {
            name: "rare".into(),
            actions: vec!["step".into()],
            occurrences: 1,
            success_rate: 1.0,
            avg_duration_ms: 50.0,
        };
        let result = crystallizer.crystallize(&obs);
        assert!(!result.success);
        assert!(result.reason.contains("Not enough"));
    }

    #[test]
    fn test_low_success_rate_rejected() {
        let crystallizer = SkillCrystallizer::default();
        let obs = PatternObservation {
            name: "flaky".into(),
            actions: vec!["step".into()],
            occurrences: 20,
            success_rate: 0.3,
            avg_duration_ms: 50.0,
        };
        let result = crystallizer.crystallize(&obs);
        assert!(!result.success);
        assert!(result.reason.contains("too low"));
    }

    #[test]
    fn test_duplicate_prevention() {
        let crystallizer = SkillCrystallizer::default();
        let obs = good_observation();
        assert!(crystallizer.crystallize(&obs).success);
        assert!(!crystallizer.crystallize(&obs).success);
    }
}
